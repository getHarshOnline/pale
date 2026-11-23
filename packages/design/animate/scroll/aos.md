+++
description = "Use it to decide whether to keep AOS, how its versions differ and how to fix its failure modes."
uses = [{ scope = "local", resource_id = "animate/scroll/guide" }]
see_also = [{ scope = "local", resource_id = "animate/scroll/native-scroll" }, { scope = "local", resource_id = "animate/expressive-motion" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "access/web" }]

[guide]
exposure = "internal"
+++

# Keep an AOS reveal accessible and usable

AOS adds classes to elements with `data-aos` attributes as they scroll into view; its stylesheet does the animating. Browser mechanisms come first ([Use browser scroll timelines and arrival observers](native-scroll.md)); this file explains when to keep AOS, how its versions differ, and how to fix their failure modes.

## Contents

1. When AOS fits
2. Versions
3. Defects and their mitigations
4. Detection

## 1. When AOS fits

- Use it for a one-time reveal when that entrance helps a narrative or sequence. Apply the relevant mitigations below.
- Keep it when the page already uses it and replacing it would exceed the repair. An Intersection Observer and a class change do the same job with no dependency and none of the defects.
- **AOS with an ES-module-only loader.** Neither version is a self-contained ES module: 2.3.4 ships UMD only, and the 3.0 beta's ES build imports bare specifiers (`lodash.throttle`, `lodash.debounce`). Under the ES-module-only profile it works only through a recorded wrapping step with its version, hash and licence beside it (the project’s module loading and mount guide).
- Its bundled animation names (fade, slide, zoom, flip) are a catalogue, not a vocabulary. Which property moves is decided by the role contract in [Give expressive motion a purpose, “The material event contract”](../expressive-motion.md#the-material-event-contract), never by picking an entry; fade-up and slide entries are the same-entrance tell ([Inspect an interface in its rendered context](../../inspect.md)).

## 2. Versions

|  | 2.3.4 | 3.0.0-beta |
| --- | --- | --- |
| Start states | `[data-aos]` start states (`opacity: 0`, offset or scaled transform) apply unconditionally; no no-js gate | Start states apply under `html:not(.no-js)` |
| Pointer events | Not removed | `[data-aos]` has `pointer-events: none` until animated |
| Replay | `once` | `once`, `mirror` (animate out when scrolled past) |
| Classes and events | Fixed `aos-init` and `aos-animate` classes | `initClassName`, `animatedClassName`, `useClassNames`; `aos:in` and `aos:out` events, `aos:in:<id>` for `data-aos-id` |
| Module format | UMD | CommonJS and an ES build with bare imports |

Read the installed version from the vendored file's recorded version before applying any row.

## 3. Defects and their mitigations

| Defect | Version | Consequence | Mitigation |
| --- | --- | --- | --- |
| Start states with no script gate | 2.3.4 | Every animated element is invisible when scripts are off or fail | The theme stylesheet overrides the start states to the resting state unless a script-set class is present on the root; the module that initialises AOS sets that class |
| Start states under `html:not(.no-js)` | 3.0 beta | A root without `no-js` hides every animated element when scripts are off or fail | Ship `class="no-js"` on the root and remove it only in the module that successfully imports and initialises AOS |
| `pointer-events: none` until animated | 3.0 beta | Links and controls in a not-yet-revealed element are inert | Override so pointer events are never removed ([Make the browser task accessible](../../access/web.md)); reveal on focus as in [Use browser scroll timelines and arrival observers, “2. One-time arrival with Intersection Observer”](native-scroll.md#2-one-time-arrival-with-intersection-observer) |
| No reduced-motion handling | both | Travel, zoom and flip run for readers who asked for less motion | Pass `disable` a function returning the reduce state of the motion-preference contract (the project’s stylesheet and token guide), so in the reduced state the region is already arrived, an instant state; on a `motion-preference` event, strip the attributes the same way |
| Timing only as generated attribute steps | both | Token times cannot be expressed; numbers creep into markup | No timing attributes; set `transition-duration` and `transition-timing-function` on `[data-aos]` from the role's token custom properties |
| Travel distance is a stylesheet constant | both | An untokenised distance moves content | Define the permitted reveal on a custom `data-aos` name whose properties come from the role contract |
| `once: false`, `mirror` | both / 3.0 beta | Content animates again on every pass | `once: true`, `mirror: false` |
| Elements added after init are never observed | both | A region that arrives with an island or a disclosure stays at its start state | Refresh after any DOM change the module makes, or use the browser observer, which needs no refresh ([Use browser scroll timelines and arrival observers, “2. One-time arrival with Intersection Observer”](native-scroll.md#2-one-time-arrival-with-intersection-observer)) |

With these mitigations in place, what remains of AOS is a trigger that adds a class; that is the strongest argument for the native pattern.

## 4. Detection

- `[data-aos]` where the entrance adds no meaning, or on most sections of a page.
- 2.3.4: no stylesheet override of the start states, so content is hidden without scripts.
- 3.0 beta: no `no-js` class on the root, `no-js` removed before AOS has initialised, or `pointer-events: none` left in effect on any `[data-aos]` element.
- `init` without a reduced-motion `disable` function.
- Any `data-aos-duration`, `data-aos-delay` or `data-aos-easing` attribute.
- A built-in animation name used as the design decision.
- AOS imported by package name or from a URL instead of a wrapped, vendored module.

## Worked example

An existing page uses AOS 2.3.4 for one figure's entrance. Without JavaScript, verify that the stylesheet leaves the figure visible. Enable the initialized enhancement and tab to the figure's link before scrolling. The link should work and its focus be visible. Turn on reduced motion and revisit the page. The composed figure should already be present. Check that adding content after startup does not strand it in a hidden state.

## Sources

- [AOS repository (default branch is the 3.0 beta `next`)](https://github.com/michalsnik/aos): the 3.0 beta README's settings, events and caveats, and the stylesheet sources that gate start states and pointer events.
- [AOS 2.x branch](https://github.com/michalsnik/aos/tree/v2): the 2.3.4 documentation and the stylesheet that sets start states unconditionally.
- [AOS package metadata on the npm registry](https://registry.npmjs.org/aos): the `dist-tags` (`latest` 2.3.4, `next` 3.0.0-beta) and each version's `main` and `module` entries before adopting or pinning it.
- [MDN: prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion): the query the `disable` function evaluates through `matchMedia`.
- [MDN: pointer-events](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/pointer-events): why `none` makes an element and its links unreachable by pointer.
