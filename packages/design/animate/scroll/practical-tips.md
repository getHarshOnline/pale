+++
description = "Use it to gate, write and verify anything that moves as the reader scrolls, with a check for each fault."
uses = [{ scope = "local", resource_id = "animate/scroll/guide" }]
see_also = [{ scope = "local", resource_id = "animate/scroll/native-scroll" }, { scope = "local", resource_id = "animate/scroll/aos" }, { scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "animate/motion-toolchain" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "responsive" }, { scope = "local", resource_id = "web" }]

[guide]
exposure = "internal"
+++

# Diagnose a scroll effect or arrival

Field practice for gating, writing and verifying anything that moves as the reader scrolls. Tips are the entry point, read at the decision they serve; each ends with a check to run in the browser or the source, and a **Read:** line to the reference that holds the rule. The gate lives in this skill's [Build a scroll-linked effect or one-time arrival](guide.md), the mechanisms in [Use browser scroll timelines and arrival observers](native-scroll.md) and [Keep an AOS reveal accessible and usable](aos.md); whether a moment may move at all is owned by [Design motion that helps people use the interface](../../animate.md) and [Choose a visual direction from the actual brief](../../art-direction.md). These tips never override them.

## Contents

- Someone asks for a reveal
- Writing a scroll-linked effect
- Writing a one-time arrival
- Reviewing it

## Someone asks for a reveal

### Mark the one region, not the page

A reveal on every section is the same-entrance tell, and it makes a reading page perform. Follow the brief’s expressive range and mark the regions whose arrival helps the content with `data-reveal`. **Check:** in the rendered page, count the `data-reveal` regions; they are the few the plan named, never most sections. **Read:** [Build a scroll-linked effect or one-time arrival, “Before writing”](guide.md#before-writing); [Choose a visual direction from the actual brief](../../art-direction.md); [Inspect an interface in its rendered context](../../inspect.md).

### Prefer CSS for anything tied to scroll position

A scroll handler that seeks an animation on every event competes with every other script on the main thread and stutters under load; a CSS scroll timeline animating transform or opacity keeps tracking the scrollbar. Use `animation-timeline: view()` or `scroll()` inside `@supports`. **Check:** in a browser without scroll-driven animation, the element shows its final state; in one with it, the effect tracks scroll with no script running. **Read:** [Use browser scroll timelines and arrival observers, “1. Scroll-linked progress with CSS”](native-scroll.md#1-scroll-linked-progress-with-css); [Choose the lightest animation mechanism, “2. Choose by job”](../motion-toolchain.md#2-choose-by-job).

## Writing a scroll-linked effect

### Declare the timeline after the shorthand

`animation-timeline` is not part of the `animation` shorthand, and the shorthand resets it, so a timeline declared first is silently lost and the effect plays on time instead. Write the `animation` shorthand, then `animation-timeline`, then `animation-range`. **Check:** DevTools computed style shows the `view()` or `scroll()` timeline on the element; the effect does not play on load before any scroll. **Read:** [Use browser scroll timelines and arrival observers, “1. Scroll-linked progress with CSS”](native-scroll.md#1-scroll-linked-progress-with-css).

### Set the range by name and check it in a visualiser

A percentage guessed for where an effect starts lands differently for a region taller or shorter than the viewport. Choose the named range that states the intent (`entry` for arriving, `cover` for the whole passage, `contain` while fully visible), add offsets only after seeing it, and preview the range in a view-timeline range visualiser. **Check:** at the narrowest and the widest width, the effect starts and ends where the plan says, for the tallest and the shortest marked region. **Read:** [Use browser scroll timelines and arrival observers, “1. Scroll-linked progress with CSS”](native-scroll.md#1-scroll-linked-progress-with-css).

### Name the timeline when the scroller is not the parent

The `nearest` lookup only considers elements that affect the subject's position and size, so an absolutely positioned subject can skip its visual scroller and bind to another. Give the scroller a `scroll-timeline` or `view-timeline` name, and use `timeline-scope` on a shared ancestor when the animated element sits outside it. **Check:** scroll the intended container alone; the effect tracks it, and scrolling the page does not move it. **Read:** [Use browser scroll timelines and arrival observers, “1. Scroll-linked progress with CSS”](native-scroll.md#1-scroll-linked-progress-with-css).

## Writing a one-time arrival

### Implement the selected arrival

Use the reference and world to choose the carrier; the native example demonstrates depth but does not require blur. A selected translation or mask can be appropriate. Keep words and controls readable while decorative layers arrive. **Check:** frame review matches the intended choreography, preserves legibility and meets its budget. **Read:** [Use browser scroll timelines and arrival observers, “2. One-time arrival with Intersection Observer”](native-scroll.md#2-one-time-arrival-with-intersection-observer).

### Keep content there without scripts

A start state written in the stylesheet hides content whenever the module fails to load. Let the module add `reveal-pending` only to marked regions below the fold; reveal on `focusin` and `unobserve` after the first arrival. **Check:** with scripts off every region is visible; tabbing into a pending region reveals it at once. **Read:** [Use browser scroll timelines and arrival observers, “2. One-time arrival with Intersection Observer”](native-scroll.md#2-one-time-arrival-with-intersection-observer); [Make the browser task accessible](../../access/web.md).

### Size the threshold against the tallest region

The threshold is a fraction of the region that must be visible; a region taller than the viewport divided by that fraction never reaches it and stays pending. Check the tallest marked region at the shortest viewport before trusting the module's arrival threshold; where it cannot reach it, the region takes no reveal or the constant's stated meaning is revised. **Check:** on a landscape phone viewport, scroll through the tallest marked region; it arrives. **Read:** [Use browser scroll timelines and arrival observers, “2. One-time arrival with Intersection Observer”](native-scroll.md#2-one-time-arrival-with-intersection-observer); [Recompose an interface without losing its task](../../responsive.md).

### Leave AOS for the native pattern where it can go

Once every mitigation is in place, AOS is a trigger that adds a class, carrying hidden start states, removed pointer events and a module format that needs wrapping. Where AOS is already a dependency and replacing it is in scope, swap it for the observer module and the reveal sample. **Check:** with AOS in place, run the page with scripts off and tab through a pending region; any hidden or inert content is a defect to mitigate before anything else. **Read:** [Keep an AOS reveal accessible and usable, “3. Defects and their mitigations”](aos.md#3-defects-and-their-mitigations); [Use browser scroll timelines and arrival observers, “2. One-time arrival with Intersection Observer”](native-scroll.md#2-one-time-arrival-with-intersection-observer).

## Reviewing it

### Scroll fast and slow, then slow the arrival down

A reveal that looks right at reading speed can flash, replay or lag when the reader scrolls fast. Scroll past every marked region quickly, then slowly, then jump by an anchor link and by Back; slow the arrival in the browser's animation inspector and step it frame by frame. **Check:** each region arrives once, never replays on scroll-up, is in its final state after an anchor jump, and never waits behind the reader. **Read:** [Use browser scroll timelines and arrival observers, “3. Detection”](native-scroll.md#3-detection); [Inspect an interface in its rendered context](../../inspect.md).

### Scroll the reduced state as its own page

Switching off the effect is sufficient only if the composed state retains its information. Check that progress remains legible and the intended emphasis survives. Scroll the page with `data-motion="reduce"`, then with only the system preference set, at reading speed. **Check:** in both, each marked region is complete, through an in-place change or its composed still, the scroll-linked figure still shows the reader's stage, and nothing travels. **Read:** [Use browser scroll timelines and arrival observers, “1. Scroll-linked progress with CSS”](native-scroll.md#1-scroll-linked-progress-with-css); [Choose a visual direction from the actual brief](../../art-direction.md).

### Scroll it under a throttled CPU

Scroll effects that run on the main thread fall behind when scripts are busy, which is the reader's phone on a heavy page. Throttle the CPU in the browser's performance panel and scroll the page again. **Check:** CSS scroll-linked effects stay locked to the scrollbar; profile any lagging effect and choose a supported mechanism. CSS keyframes can express visual holds; media decoding and custom frame orchestration may still need an island whose listener records position ([Carry a design through the web build](../../web.md)). **Read:** [Use browser scroll timelines and arrival observers, “1. Scroll-linked progress with CSS”](native-scroll.md#1-scroll-linked-progress-with-css); [Carry a design through the web build](../../web.md).

## Sources

- [MDN: CSS scroll-driven animations](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Scroll-driven_animations): scroll and view timelines and feature detection.
- [MDN: scroll()](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/animation-timeline/scroll): the `nearest` scroller default and an inactive timeline on an axis without scrollbars.
- [MDN: Timeline range names](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Scroll-driven_animations/Timeline_range_names): what `entry`, `cover`, `contain` and the crossings measure.
- [Chrome for Developers: Animate elements on scroll with scroll-driven animations](https://developer.chrome.com/docs/css-ui/scroll-driven-animations): `animation-timeline` outside the shorthand, compositor-only properties, named ranges, `timeline-scope` and the `nearest` lookup skipping positioned ancestors.
- [Chrome for Developers: A case study on scroll-driven animations performance](https://developer.chrome.com/blog/scroll-animation-performance-case-study): scroll timelines unaffected by heavy main-thread work while scroll handlers stall.
- [View timeline ranges visualiser](https://scroll-driven-animations.style/tools/view-timeline/ranges/): previewing where each named range starts and ends for a subject.
- [MDN: Intersection Observer API](https://developer.mozilla.org/en-US/docs/Web/API/Intersection_Observer_API): `threshold` as a fraction of the target, `rootMargin`, `unobserve` and the main-thread callback.
- [AOS repository](https://github.com/michalsnik/aos): the start states, pointer events and settings that the mitigations answer.
