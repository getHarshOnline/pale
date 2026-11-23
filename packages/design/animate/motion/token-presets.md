+++
description = "Use it to turn shared motion tokens into the options of a Motion call in an island."
uses = [{ scope = "local", resource_id = "animate/motion/guide" }]
see_also = [{ scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/animejs/guide" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "animate/temporal-hierarchy" }, { scope = "local", resource_id = "animate/read-tokens" }, { scope = "local", resource_id = "animate/feedback-roles" }]

[guide]
exposure = "internal"
+++

# Convert design tokens to Motion options

How motion tokens reach Motion calls in an island. The rule for shared design-role values, how islands read them and the spring policy live in [Read timing and spring roles from the system, “2. How islands take their values and the spring policy”](../read-tokens.md#2-how-islands-take-their-values-and-the-spring-policy); the meaning of each role in [Choose feedback roles and timing, “1. The role contract”](../feedback-roles.md#1-the-role-contract); values in the consuming project’s token source. The CSS spring pattern is [Carry a design through the web build](../../web.md). This file covers the conversion to Motion options.

The names, custom-property output, `data-motion` attribute and `motion-preference` event below belong to one worked theme. Adapt them to the project’s declared interface. These helpers assume nonempty, valid CSS times and cubic-bezier values; validate token output at its source and handle missing values before calling the library. They are conversion examples, not a general CSS parser.

## Contents

1. Reading tokens at runtime
2. Role to token map
3. Reduced motion
4. Detection

## 1. Reading tokens at runtime

The token build emits every motion token as a custom property on the world and mode scope; reading those build outputs is how an island takes its numbers from tokens without restating them. An island reads the computed value from its own element, so the world, palette and mode in force at that point of the page are the ones it uses:

```js
const read = (el, name) =>
  getComputedStyle(el).getPropertyValue(`--${name}`).trim();
const seconds = (v) =>
  v.endsWith("ms") ? parseFloat(v) / 1000 : parseFloat(v);
const bezier = (v) => (v.match(/-?\d*\.?\d+/g) ?? []).map(Number);

export const tweenSeconds = (el, role, easing = "easing-productive") => ({
  duration: seconds(read(el, `duration-${role}`)),
  ease: bezier(read(el, easing)),
});

export const springPreset = (el, prefix) => ({
  type: "spring",
  visualDuration: seconds(read(el, `${prefix}-visual-duration`)),
  bounce: parseFloat(read(el, `${prefix}-bounce`)),
});
```

- Motion takes seconds; token durations arrive as CSS times (`ms` or `s`). The conversion by 1000 is a unit change, not a design value. Helper names carry the unit (`tweenSeconds`); the [Build a page animation with anime.js](../animejs/guide.md) helpers carry milliseconds (`tweenMs`), so a helper copied between engines cannot pass the wrong unit silently.
- Custom property values of `var()` references resolve in the computed value, so an aliased token (for example a decay aliased to the expressive duration) reads as its final time.
- Read once when the island mounts and again only when the world, mode or motion setting changes; never inside an animation frame.
- An empty string means the token is missing in this scope: report it as a token gap and do not substitute a number ([Choose a visual direction from the actual brief](../../art-direction.md)).

## 2. Role to token map

Custom property names follow the token ids with dots replaced by hyphens.

| Role (defined in [Design motion that helps people use the interface](../../animate.md)) | Motion options | Reduced equivalent |
| --- | --- | --- |
| Feedback, preview, enter, leave, result, sequence | `tweenSeconds(el, role)` → `duration-<role>` with `easing-productive` | `motion-reduced-<role>` |
| An on-screen move (a sliding indicator, a reordered item) | `tweenSeconds(el, role, "easing-move")` | `motion-reduced-<role>` |
| Hover intent before a preview opens | `delay-preview`, pointer only; focus opens at once ([Make the browser task accessible](../../access/web.md)) | none: open on focus or press |
| A productive settle (a highlight, a drawer) | `springPreset(el, "motion-spring-productive")` | the role's `motion-reduced-<role>` |
| An expressive moment | `expressive(el)` (below): `duration-expressive` × `world-motion-expressive-scale`, with `ease` from `world-motion-expressive-easing`; or `springPreset(el, "world-motion-expressive-spring")` | `reducedExpressive(el)` (below): `motion-reduced-expressive` × `world-motion-reduced-expressive-scale`, easing `world-motion-reduced-expressive-easing`, settling on the composed still |
| A productive stagger (steps where order is information) | `stagger(seconds(read(el, "delay-stagger")))` passed as `delay` | `motion-reduced-stagger`: no cascade, the elements appear together |
| An expressive stagger | `expressiveStagger(el)` (below): `delay-stagger` × `world-motion-expressive-scale`, passed to `stagger()` | `motion-reduced-stagger` |
| Emission release to rest | `world-motion-decay` with the world's expressive easing (behaviour in [Choose what material and light communicate](../../surfaces.md)) | `motion-reduced-decay` |

The scale multiplication is the composition used by this example ([Read timing and spring roles from the system, “2. How islands take their values and the spring policy”](../read-tokens.md#2-how-islands-take-their-values-and-the-spring-policy)); other shared role values are used as read, after unit conversion:

```js
const scale = (el) => parseFloat(read(el, "world-motion-expressive-scale"));

export const expressive = (el) => ({
  duration: seconds(read(el, "duration-expressive")) * scale(el),
  ease: bezier(read(el, "world-motion-expressive-easing")),
});

export const expressiveStagger = (el) =>
  seconds(read(el, "delay-stagger")) * scale(el);

export const reducedExpressive = (el) => ({
  duration:
    seconds(read(el, "motion-reduced-expressive")) *
    parseFloat(read(el, "world-motion-reduced-expressive-scale")),
  ease: bezier(read(el, "world-motion-reduced-expressive-easing")),
});
```

The reduced state takes the second scale for the same reason the full state takes the first: a world without its own pace in the reduced state reads as every other world with the motion removed ([Design the reduced-motion alternative, “Translate, don't delete”](../reduced-motion.md#translate-dont-delete)).

- Productive roles never read `world-motion-*`; expressive moments never borrow productive presets.
- Exit always takes the leave role, which is shorter than enter ([Show what happens first, “Out faster than in”](../temporal-hierarchy.md#out-faster-than-in)).
- The world's expressive curve is part of its identity ([Choose a visual direction from the actual brief](../../art-direction.md)); an island that flattens it to the productive easing loses the world.

## 3. Reduced motion

The reduced state is one of the four presentation states an island renders, authored like the full one: each role plays its reduced equivalent from the map above, with its own keyframes, with the final state and cleanup explicit even when the alternative is immediate. What each form is: [Design the reduced-motion alternative, “Translate, don't delete”](../reduced-motion.md#translate-dont-delete); its floor: [Make the browser task accessible](../../access/web.md). The code reads the motion preference; the attribute and the `motion-preference` event are the project’s stylesheet and token guide.

```js
const root = document.documentElement;
const systemReduces = matchMedia("(prefers-reduced-motion: reduce)");
export const reduced = () =>
  root.dataset.motion === "reduce" ||
  (root.dataset.motion !== "allow" && systemReduces.matches);

const onChange = () => {
  running?.complete();
  rebuild(reduced());
}; // valid when the animation end is the intended still
systemReduces.addEventListener("change", onChange);
document.addEventListener("motion-preference", onChange);
const dispose = () => {
  systemReduces.removeEventListener("change", onChange);
  document.removeEventListener("motion-preference", onChange);
  running?.cancel();
};

// An expressive arrival in each state: scale and opacity in full; the opacity step alone when reduced.
const arrive = (el) =>
  reduced()
    ? animate(el, { opacity: [0, 1] }, reducedExpressive(el))
    : animate(
        el,
        { opacity: [0, 1], scale: [parseFloat(read(el, "scale-enter")), 1] },
        expressive(el),
      );
```

- Use `complete()` when the animation end is the intended reduced state; otherwise set that state explicitly. The island then returns to its still or plays the reduced presets (the project’s module loading and mount guide: the mount stays in place when the preference changes).
- Stylesheet transitions in the reduced state follow [Carry a design through the web build](../../web.md).

The component supplies `running`, `rebuild` and the animation import. Call `dispose()` when it leaves the page and restore the intended still if the host stays mounted.

## 4. Detection

- Shared-role values copied into calls instead of read from their source; distinguish authored scene timing.
- A local multiplier that changes the intended shared role without a corresponding choice in the system.
- Tokens read from the document root instead of the island's element, so a world scope is ignored.
- A reduced path that multiplies durations toward zero instead of selecting reduced presets, or one that ignores `data-motion`.

## Sources

- [MDN: getComputedStyle](https://developer.mozilla.org/en-US/docs/Web/API/Window/getComputedStyle): reading resolved custom property values from an element.
- [MDN: prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion): the media feature and listening for its changes through `matchMedia`.
- [Motion: spring](https://motion.dev/docs/spring): the duration-based spring options `visualDuration` and `bounce` that `springPreset` fills.
