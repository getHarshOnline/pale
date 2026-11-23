+++
description = "Use it to choose imports, targets and tween parameters once the movement and its clock are decided."
uses = [{ scope = "local", resource_id = "animate/animejs/api-v4" }]
see_also = [{ scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/read-tokens" }]

[guide]
exposure = "internal"
+++

# Animate and tween with anime.js

Choose imports, targets and tween parameters.

## 1. Imports

anime.js 4 is a set of named ES module exports; there is no global `anime` function.

| Export | Use |
| --- | --- |
| `animate` | Animate targets |
| `createTimeline` | Sequence animations, calls and labels |
| `stagger` | Distribute delays or values across targets |
| `svg` | `createDrawable`, `morphTo`, `createMotionPath` |
| `utils` | `set`, `get`, `remove`, `$` and numeric helpers |
| `engine` | Global clock settings |
| `createScope` | Media-query-aware grouping with one `revert()` |
| `onScroll` | Scroll observer for scroll-synchronised playback |
| `cubicBezier`, `linear`, `spring`, `eases` | Ease functions (also grouped as `easings`); `spring()` from 4.2, `createSpring()` in 4.0–4.1. The example sampled-spring policy uses the emitted easing instead ([Read timing and spring roles from the system, “2. How islands take their values and the spring policy”](../read-tokens.md#2-how-islands-take-their-values-and-the-spring-policy)) |

The package also exposes subpath entries (for example `animejs/svg`, `animejs/easings`). In a theme with the same module and markup rules every import resolves through the vendored module (the project’s module loading and mount guide).

## 2. `animate()`

```js
const animation = animate(targets, { property: value, duration, delay, ease });
```

- **Targets:** a selector, an element, an array or a plain object. Select within the island's mount so nothing outside it moves.
- **Properties:** CSS properties, individual transforms (`x`, `y`, `scale`, `rotate` and axis forms), CSS variables, SVG and DOM attributes, and object properties. Animate transforms and opacity for movement ([Carry a design through the web build](../../web.md)).
- **Tween values:** a target value; `[from, to]`; an object `{ from, to }`; a relative value (`'+=…'`); a function of the target and index. Units carried by the value.
- **Keyframes:** an array of values on one property, or a `keyframes` list of property objects.
- **Per-property parameters:** an object value can carry its own `duration`, `delay`, `ease` and `composition`.

## Sources

- [anime.js documentation: Installation](https://animejs.com/documentation/getting-started/installation): the named exports, subpath modules and the ES module bundle file.
- [anime.js documentation: Animation](https://animejs.com/documentation/animation): targets, tween values, keyframes, playback settings, callbacks and methods.
