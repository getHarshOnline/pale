+++
description = "Use it when a Motion call in an island misbehaves, to find the check and the reference for the fault."
uses = [{ scope = "local", resource_id = "animate/motion/guide" }]
see_also = [{ scope = "local", resource_id = "animate/motion/islands-api" }, { scope = "local", resource_id = "animate/motion/token-presets" }, { scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "animate/motion-toolchain" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/scroll/native-scroll" }, { scope = "local", resource_id = "controls" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "animate/scroll/guide" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "animate/retarget-controls" }, { scope = "local", resource_id = "animate/pause-and-resume" }, { scope = "local", resource_id = "animate/read-tokens" }]

[guide]
exposure = "internal"
+++

# Diagnose a Motion animation

Field practice for writing Motion calls in an island. Tips are the entry point, read at the call they serve; each ends with a check to run in the browser or the source, and a **Read:** line to the reference that holds the rule. The call shapes live in [Use the Motion JavaScript API in a component](islands-api.md) and the token conversion in [Convert design tokens to Motion options](token-presets.md); whether anything moves, and how fast, is owned by [Design motion that helps people use the interface](../../animate.md). These tips never override either.

## Contents

- Starting an island
- Moving things
- Interrupting, suspending and settling
- Reviewing the motion

## Starting an island

### Try the browser before vendoring an engine

A one-property island with no spring, sequence or gesture runs on the Web Animations API that Motion itself is built on; vendoring a module for it adds bytes and a pin to maintain. Write it as `element.animate()` with the token easing string and a millisecond duration read from the token; reach for Motion only when the island needs springs, sequences, gestures or `inView()`, and prefer the mini build while the island animates only opacity, full transforms or registered custom properties. **Check:** in source, the island imports no motion module unless it uses one of those features, and imports the hybrid build only for independent transforms, sequences or unregistered variables. **Read:** [Use the Motion JavaScript API in a component, “1. Builds and imports”](islands-api.md#1-builds-and-imports); [Choose the lightest animation mechanism, “2. Choose by job”](../motion-toolchain.md#2-choose-by-job).

### Read tokens from the island's own element

Tokens read from the document root ignore the world and mode scope the island sits in, so an island inside an enclave moves like its host. Call `tweenSeconds(el, role)` and `springPreset(el, prefix)` with the island's element, once at mount and again only when the world, mode or motion setting changes. **Check:** switch the colour mode or motion setting and remount; the island picks up the new values, and no read happens inside an animation frame. **Read:** [Convert design tokens to Motion options, “1. Reading tokens at runtime”](token-presets.md#1-reading-tokens-at-runtime); [Read timing and spring roles from the system, “2. How islands take their values and the spring policy”](../read-tokens.md#2-how-islands-take-their-values-and-the-spring-policy).

## Moving things

### Animate one `transform` string

Motion's independent `x`, `y` and `scale` are animated through CSS variables and are not hardware-accelerated, so plain movement written that way costs main-thread frames. Animate `transform` as one string, plus opacity; keep independent transforms for two axes that must animate separately, and put their measured frame cost in the existing review. **Check:** in a performance trace of the running island, movement frames show no style recalculation of the island's descendants. **Read:** [Use the Motion JavaScript API in a component, “2. `animate()`”](islands-api.md#2-animate); [Carry a design through the web build](../../web.md).

### Use the project’s spring model

Physics parameters typed into a call are literals the token set cannot change, and duration-based springs ignore any velocity handed to them. The example uses `springPreset(el, "motion-spring-productive")` or the world’s expressive spring. A momentum gesture instead needs the project’s physics model and parameters; resolve that choice at its shared owner rather than copying demo values. **Check:** the chosen preset follows the project’s policy; a momentum release carries its actual velocity and settles without a jump. **Read:** [Use the Motion JavaScript API in a component, “4. Springs”](islands-api.md#4-springs); [Read timing and spring roles from the system, “2. How islands take their values and the spring policy”](../read-tokens.md#2-how-islands-take-their-values-and-the-spring-policy).

### Hand `scroll()` an animation, not a callback

Where the browser supports scroll timelines, Motion runs an animation passed to `scroll()` on that timeline, hardware-accelerated and smooth under a busy main thread; a progress callback runs script on every scroll. Build the animation with `animate()` from tokens and pass it; keep a callback only for a value no style can carry, and only where scroll position is the island's meaning. **Check:** in source, `scroll()` receives an animation; with the CPU throttled in the performance panel, the effect still tracks the scrollbar. **Read:** [Use the Motion JavaScript API in a component, “6. `inView()` and `scroll()`”](islands-api.md#6-inview-and-scroll); [Use browser scroll timelines and arrival observers, “1. Scroll-linked progress with CSS”](../scroll/native-scroll.md#1-scroll-linked-progress-with-css).

### Let `press()` tell a completed press from a slide-off

A raw pointer listener fires on a right click and on a press that slid off the control. `press()` filters secondary pointers and reports `success`, so an island acts only when the press completed on its element. Use it only on the island's own native control, which already carries focus and Enter; the pressed look stays in CSS. **Check:** press and release on the control: it acts; right-click it, or press and slide off: it does not; tab to it and press Enter: it still acts. **Read:** [Use the Motion JavaScript API in a component, “7. `hover()` and `press()`”](islands-api.md#7-hover-and-press); [Design controls that work through the whole task](../../controls.md).

## Interrupting, suspending and settling

### Retarget with a new `animate()`, never `cancel()`

`cancel()` snaps back to where the animation started, so an interrupted state change jumps. On new input, start a new `animate()` toward the new target; it begins from the current value. Keep only the latest controls. **Check:** trigger the change and reverse it halfway, repeatedly; the element turns around from where it is, without a jump. **Read:** [Use the Motion JavaScript API in a component, “5. Playback controls and interruption”](islands-api.md#5-playback-controls-and-interruption); [Reverse or retarget a control in flight, “Rules”](../retarget-controls.md#rules).

### Return a leave function from `inView()`

Without a returned function the callback fires once and the island keeps running offscreen; with one, it runs on leave and the callback fires again on each entry. Start work in the callback and return a function that stops or pauses the controls.

Two jobs use this one function and behave differently, so the job is decided before it is called. **Suspending a running island** wants the leave function: work stops offscreen and resumes at its current state. **A one-time arrival** must not run twice, so it stops observing the element once it has arrived and returns nothing ([Build a scroll-linked effect or one-time arrival, “Core rules”](../scroll/guide.md#core-rules)). **Check:** scroll the island away while it moves; the browser's animation inspector shows nothing running for it, and scrolling back shows its current state without replaying an entrance. **Read:** [Use the Motion JavaScript API in a component, “6. `inView()` and `scroll()`”](islands-api.md#6-inview-and-scroll); [Pause work and resume from the same state](../pause-and-resume.md).

### Settle before reducing

A preference change mid-flight that rebuilds without settling leaves an element stranded half-way. On a preference change, settle to the current logical state, then rebuild its reduced form. Use `complete()` only when the animation end is that state; otherwise apply the intended still explicitly. **Check:** start the island, switch reduced motion on through the project’s preference control mid-animation; the island settles and then behaves reduced. **Read:** [Convert design tokens to Motion options, “3. Reduced motion”](token-presets.md#3-reduced-motion); the project’s module loading and mount guide.

## Reviewing the motion

### Slow it down with `speed`

Motion that looks right at full speed often hides an overshoot, a wrong origin or two properties out of step. During review, set the returned controls' `speed` below one, or slow the group in the browser's animation inspector, and watch entry, exit and an interruption; never ship a changed speed. **Check:** the slowed recording shows every property starting from where the element was and ending settled; in source, no `speed` assignment remains. **Read:** [Use the Motion JavaScript API in a component, “5. Playback controls and interruption”](islands-api.md#5-playback-controls-and-interruption); [Inspect an interface in its rendered context](../../inspect.md).

## Worked example

A small panel is opening when the user presses its trigger again. Read timing roles from the panel's own element. Start the reverse from the current transform with a new `animate()` call. Try the same motion with a slow playback speed. If it jumps to the opening frame before reversing, inspect the old cancellation and verify which properties the two calls own. Switch to reduced motion mid-flight and check that the final state and focus are clear.

## Sources

- [Motion: performance](https://motion.dev/docs/performance): why independent transforms are not hardware-accelerated and the full `transform` string that is.
- [Motion: animate](https://motion.dev/docs/animate): the mini and hybrid builds, playback controls, `speed` and `cancel()` semantics.
- [MDN: Element.animate()](https://developer.mozilla.org/en-US/docs/Web/API/Element/animate): the Web Animations call and its millisecond durations.
- [Motion: scroll](https://motion.dev/docs/scroll): animations passed to `scroll()` running on the browser's scroll timeline with hardware acceleration, and callbacks that do not.
- [Motion: inView](https://motion.dev/docs/inview): the leave function and the callback firing once unless one is returned.
- [Motion: press](https://motion.dev/docs/press): secondary-pointer filtering, the `success` flag and keyboard operation.
- [Motion: web animation performance tier list](https://motion.dev/magazine/web-animation-performance-tier-list): composited writes that skip paint, and inherited custom properties that restyle a tree.
- [Chrome DevTools: Inspect animations](https://developer.chrome.com/docs/devtools/css/animations): slowing and scrubbing animation groups during review.
