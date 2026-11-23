+++
description = "Use it to build scroll-driven and arrival effects with CSS scroll timelines and Intersection Observer instead of a library."
uses = [{ scope = "local", resource_id = "animate/scroll/guide" }]
see_also = [{ scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/motion-toolchain" }, { scope = "local", resource_id = "animate/expressive-motion" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/motion/guide" }, { scope = "local", resource_id = "animate/motion/practical-tips" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "animate/read-tokens" }]

[guide]
exposure = "internal"
+++

# Use browser scroll timelines and arrival observers

CSS scroll-driven animations and Intersection Observer do nearly every permitted scroll job without a library. Choose a scroll effect from the brief and the project’s expressive range, as the [Build a scroll-linked effect or one-time arrival](guide.md) describes; shared timing comes from the role’s tokens ([Read timing and spring roles from the system, “1. Which values come from tokens”](../read-tokens.md#1-which-values-come-from-tokens)). The motion-preference selector pair in the samples is the project’s stylesheet and token guide.

## Contents

1. Scroll-linked progress with CSS
2. One-time arrival with Intersection Observer
3. Detection

## 1. Scroll-linked progress with CSS

Use when the effect's state is a function of scroll position: a progress mark for a long sequence, a figure that advances as its section is read. Animating transform or opacity, it can run off the main thread; it needs no script.

```css
@supports (animation-timeline: view()) {
  .sequence-figure {
    animation: figure-advance var(--duration-sequence) linear both;
    animation-timeline: view();
    animation-range: entry cover 50%;
  }
  /* The reduced state: the same progress, carried by light instead of travel. */
  :root[data-motion="reduce"] .sequence-figure {
    animation-name: figure-advance-lit;
  }
  @media (prefers-reduced-motion: reduce) {
    :root:not([data-motion="allow"]) .sequence-figure {
      animation-name: figure-advance-lit;
    }
  }
}
```

- **The reduced state keeps the progress.** `figure-advance-lit` changes only what does not travel (the stage marks' opacity, the lit edge reaching the current stage, from the world's material tokens), so the figure still tells the reader where they are. A figure with no such carrier rests in its advanced state, an instant state that lands, and its stages stay legible in structure (numbering, position).

- `animation-timeline: scroll()` follows a scroll container's progress (`scroll(nearest block)`, `scroll(root)`); `view()` follows an element's progress through its scrollport, with an optional axis and inset. Named timelines (`scroll-timeline`, `view-timeline`, `timeline-scope`) link an animation to another element's scroll.
- `animation-range` selects the part of the timeline that drives the keyframes with named ranges (`cover`, `contain`, `entry`, `exit`, `entry-crossing`, `exit-crossing`) and offsets.
- Declare `animation-timeline` after the `animation` shorthand; the shorthand resets it. On a scroll timeline the duration is `auto` (the specified value; time has no meaning there). Where a browser still needs a time value for the shorthand to apply, any non-zero token duration maps to progress, not time.
- `linear` timing keeps progress proportional to scroll; the world's easing belongs to time-based expressive moments, not to the input mapping. Repeating the same keyframe value across a range can hold a visual state while scroll progress advances. Use a custom frame controller when the selected edit needs decoding or orchestration CSS cannot supply, not merely because it has holds ([Compose a sequence people can follow](../../films.md); [Choose the lightest animation mechanism](../motion-toolchain.md)).
- Scroll-driven animation is not available in every widely used browser, so it lives inside `@supports`; outside it the element is in its final state. The keyframes' resting state is the element's normal styled state.
- A scroll-linked effect reverses when the reader scrolls back. That suits progress; it is wrong for a one-time arrival, which uses section 2.

## 2. One-time arrival with Intersection Observer

Use when an element should arrive once as the reader reaches it and the arrival carries order. The template marks only the elements whose reveal is earned, with `data-reveal`; the script sets the pending state, so without it everything is simply there.

An authored arrival uses the design in the brief and role values ([Give expressive motion a purpose](../expressive-motion.md)). The sample below demonstrates a depth treatment, not a required style. Its animated `.reveal-surface` child is decorative; words, controls and information remain visible and usable. Choose another treatment when the reference calls for it. The reduced treatment removes travel and preserves the intended information ([Design the reduced-motion alternative](../reduced-motion.md)).

```js
// Behavioural constants of this module, not design tokens: what counts as "arrived".
const ARRIVAL_THRESHOLD = 0.25; // a quarter of the element is in view
const ARRIVAL_MARGIN = "0px 0px -10% 0px"; // and it has cleared the bottom tenth of the viewport

export function reveal(root) {
  if (!("IntersectionObserver" in window)) return;
  const listeners = new AbortController();
  const show = (el) => {
    el.classList.add("is-revealed");
    observer.unobserve(el);
  };
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (
          entry.isIntersecting &&
          entry.intersectionRatio >= ARRIVAL_THRESHOLD
        )
          show(entry.target);
      }
    },
    { threshold: ARRIVAL_THRESHOLD, rootMargin: ARRIVAL_MARGIN },
  );
  // Never blanket: only regions the template marked because their arrival is earned.
  for (const el of root.querySelectorAll("[data-reveal]")) {
    if (el.getBoundingClientRect().top < innerHeight) continue; // already in view: leave it alone
    el.classList.add("reveal-pending");
    el.addEventListener("focusin", () => show(el), {
      once: true,
      signal: listeners.signal,
    });
    observer.observe(el);
  }
  return () => {
    observer.disconnect();
    listeners.abort();
    for (const el of root.querySelectorAll(".reveal-pending"))
      el.classList.remove("reveal-pending");
  };
}
```

```css
/* The region waits one plane back, softened as the world's receding planes are, */
/* and comes forward into focus. This example uses blur and opacity. */
.reveal-pending > .reveal-surface {
  transition-property: opacity, filter;
  transition-duration: calc(
    var(--duration-expressive) * var(--world-motion-expressive-scale)
  );
  transition-timing-function: var(--world-motion-expressive-easing);
}
.reveal-pending:not(.is-revealed) > .reveal-surface {
  opacity: 0;
  filter: blur(var(--world-depth-blur));
}
:root[data-motion="reduce"] .reveal-pending > .reveal-surface {
  transition-property: opacity;
  transition-duration: calc(
    var(--motion-reduced-expressive) *
      var(--world-motion-reduced-expressive-scale)
  );
  transition-timing-function: var(--world-motion-reduced-expressive-easing);
  filter: none;
}
@media (prefers-reduced-motion: reduce) {
  :root:not([data-motion="allow"]) .reveal-pending > .reveal-surface {
    transition-property: opacity;
    transition-duration: calc(
      var(--motion-reduced-expressive) *
        var(--world-motion-reduced-expressive-scale)
    );
    transition-timing-function: var(--world-motion-reduced-expressive-easing);
    filter: none;
  }
}
@media print {
  .reveal-pending:not(.is-revealed) > .reveal-surface {
    opacity: 1;
    filter: none;
  }
}
```

- Elements already on screen at load are never hidden, so nothing flashes away and back.
- Focus entering a pending element reveals it at once, so keyboard users never tab into invisible content; the pending state never removes pointer events.
- `unobserve` after the first arrival makes it once; a reveal never replays. Call the returned cleanup when reverting or replacing this enhancement; it disconnects observation and listeners.
- Options: `root` (a scrolling ancestor), `rootMargin` (CSS margin syntax, `px` or `%`) and `threshold`. The threshold and margin are behavioural constants of the island (a fraction of the region in view, an edge it must clear), not design tokens: declare each once at the top of the module with a comment stating the meaning it encodes, never inline in the call.
- The callback runs on the main thread; keep it to a class change.
- **Depth.** The pending region sits on the world's back plane: softened by `world.depth.blur`, it gains clarity as it arrives. The blur's cost grows with its radius and the region's area; keep the marked region no larger than the arrival needs and record the frame cost ([Carry a design through the web build](../../web.md)).
- **Other treatments.** Select the world-appropriate carrier from the brief: light, translation, masking or another supported change. No depth token is not proof that the region cannot arrive; preserve readable content and test the selected treatment's cost.

- Inside a Motion island, `inView()` from [Build a page animation with Motion](../motion/guide.md) observes the same crossing; never load a second engine just for this. Its leave callback is for suspending an island that keeps running, not for an arrival: an arrival that re-runs on each entry breaks the once rule, so the callback stops observing the element once it has arrived ([Diagnose a Motion animation, “Return a leave function from `inView()`”](../motion/practical-tips.md#return-a-leave-function-from-inview)).

Each sample carries its reduced state on both paths of the selector pair; what each form is: [Design the reduced-motion alternative, “Translate, don't delete”](../reduced-motion.md#translate-dont-delete); its floor: [Make the browser task accessible](../../access/web.md) motion accessibility.

## 3. Detection

- A scroll-driven animation outside `@supports`, or with `animation-timeline` declared before the `animation` shorthand.
- Hidden or offset start states written in the stylesheet for elements the script has not marked.
- A reveal applied to elements already in view at load.
- A pending element that is inert to pointer or unrevealed on focus.
- An intended narrative arrival that lost the chosen expressive character; compare it with the brief rather than assuming a fade is always wrong.
- Repeated preset arrivals without a reason in the selected treatment.
- `data-reveal` on most regions of a page, or a selector broader than `[data-reveal]`.
- A scroll handler doing what `animation-timeline` or an observer does.
- A reduced rule that sets `animation: none` on a figure whose progress had a light carrier, or on every animated element at once.
- An observer threshold or margin typed inline in the call, or with no stated meaning.

## Sources

- [MDN: CSS scroll-driven animations](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Scroll-driven_animations): scroll and view progress timelines, the properties involved and feature detection.
- [MDN: view()](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/animation-timeline/view): the axis and inset parameters and current browser support.
- [MDN: Timeline range names](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Scroll-driven_animations/Timeline_range_names): `cover`, `contain`, `entry`, `exit` and their crossings used by `animation-range`.
- [Scroll-driven Animations specification](https://drafts.csswg.org/scroll-animations-1/): normative timeline and range behaviour when browsers disagree.
- [MDN: Intersection Observer API](https://developer.mozilla.org/en-US/docs/Web/API/Intersection_Observer_API): options, entry fields, `unobserve` and the main-thread callback caveat.
- [Motion: inView](https://motion.dev/docs/inview): the observer-based trigger inside Motion islands.
