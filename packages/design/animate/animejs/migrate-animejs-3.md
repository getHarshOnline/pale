+++
description = "Use it to translate a call written for version 3 into its anime.js 4 form."
uses = [{ scope = "local", resource_id = "animate/animejs/api-v4" }]
see_also = [{ scope = "local", resource_id = "animate/animejs/tokens-reduced-motion" }]

[guide]
exposure = "internal"
+++

# Move an anime.js 3 animation to version 4

Translate a call written for version 3.

## 6. anime.js 3 forms and their 4 equivalents

| anime.js 3 | anime.js 4 |
| --- | --- |
| `anime({ targets: t, … })` | `animate(t, { … })` |
| `anime.timeline()` | `createTimeline()` |
| `easing: 'easeOutQuad'` | `ease: 'outQuad'`; power form `'out(3)'` |
| `easing: 'spring(…)'` | the token-emitted `linear()` easing ([Use tokens and reduced motion with anime.js, “2. Springs: the emitted easing”](tokens-reduced-motion.md#2-springs-the-emitted-easing)); the library form is `spring({ bounce, duration })` (4.2+) |
| `cubicBezier` as an easing string | `cubicBezier(x1, y1, x2, y2)` function |
| `direction: 'reverse'` / `'alternate'` | `reversed: true` / `alternate: true` |
| `endDelay` | `loopDelay` (between loops only) |
| `loop` as the total number of iterations | `loop` as the number of repeats |
| `{ value, duration }` | `{ to, duration }` (and `from`) |
| `begin`, `update`, `complete`, `change` | `onBegin`, `onUpdate`, `onComplete`, `onRender` |
| `loopBegin`, `loopComplete` | `onLoop` |
| `anime.set`, `anime.get`, `anime.random`, `anime.remove` | `utils.set`, `utils.get`, `utils.random`, `utils.remove` |
| `anime.stagger` | `stagger` |
| `anime.setDashoffset` with `strokeDashoffset` | `svg.createDrawable` with the `draw` property |
| `anime.path` | `svg.createMotionPath` |
| `createSpring()` (4.0–4.1) | `spring()` (4.2+; the older name still exists and warns) |

## Worked example

A marker moves once and then repeats once. In version 3 the original call uses `loop: 2` and `complete: markReady`. In version 4 pass the marker directly to `animate`, use `loop: 1` and `onComplete: markReady`. Watch both traversals and check that the callback only fires at the expected end. A direct rename of the old loop field would run an extra traversal.

## Sources

- [anime.js wiki: Migrating from v3 to v4](https://github.com/juliangarnier/anime/wiki/Migrating-from-v3-to-v4): the full list of renamed parameters, callbacks and helpers when converting older code; written for 4.0, so it names `createSpring()` where 4.2+ uses `spring()`.
- [anime.js release v4.2.0](https://github.com/juliangarnier/anime/releases/tag/v4.2.0): the `createSpring()` deprecation, its replacement `spring()` and the `bounce` and `duration` parameters.
