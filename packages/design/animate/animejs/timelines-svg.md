+++
description = "Use it when order is information, to build timelines, time positions, staggers and drawn geometry with anime.js 4."
uses = [{ scope = "local", resource_id = "animate/animejs/guide" }]
see_also = [{ scope = "local", resource_id = "animate/temporal-hierarchy" }, { scope = "local", resource_id = "animate/animejs/tokens-reduced-motion" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "animate/time-models" }]

[guide]
exposure = "internal"
+++

# Sequence steps and draw SVG with anime.js

The anime.js 4 tools for ordered motion. Order must be information before any of them is used ([Show what happens first, “Order encodes causality”](../temporal-hierarchy.md#order-encodes-causality)). Shared role timing reads tokens ([Use tokens and reduced motion with anime.js](tokens-reduced-motion.md)); authored scene action follows its selected temporal treatment ([Compose and check a live visual region](../../figures.md)).

## Contents

1. `createTimeline()`
2. Time positions
3. `stagger()`
4. Geometry built in the mount slot
5. Drawing and morphing
6. Detection

## 1. `createTimeline()`

```js
const tl = createTimeline({ defaults: { ease }, autoplay: false })
  .label("ends")
  .add(nodes, { opacity: [0, 1], duration: step }, "ends")
  .add(edge, { draw: ["0 0", "0 1"], duration: step }, "<");
```

- Parameters: `defaults` (shared animation parameters for children), `delay`, `loop`, `alternate`, `autoplay`, `playbackEase`, callbacks.
- Methods: `add(targets, params, position)` for animations or timers; `set(targets, props, position)` for instant values; `call(fn, position)` for a callback at a point; `label(name, position)`; `sync(animation, position)` to place another timeline or animation; `remove()`.
- Set shared role defaults from tokens at construction. Select authored scene curves and property timing from the scene treatment; a timeline default does not choose every child's movement.
- Timelines group coordinated work; their count does not measure expressive intensity. Separate components may own separate timelines. Judge page attention through [Choose a visual direction from the actual brief](../../art-direction.md) and film rhythm through the native composition, with one writer per animated property.

## 2. Time positions

| Position          | Places the child                                |
| ----------------- | ----------------------------------------------- |
| a number          | at that time                                    |
| `'+=n'` / `'-=n'` | after / before the end of the previous child    |
| `'*=f'`           | at a fraction of the previous child's duration  |
| `'<'`             | at the end of the previous child                |
| `'<<'`            | at the start of the previous child              |
| `'<<+=n'`         | an offset after the start of the previous child |
| a label           | at that label                                   |
| `stagger(n)`      | spread across the targets                       |

Shared interface offsets derive from role tokens. Scene offsets express the authored temporal treatment. Labels name relations ("ends", "edge", "result") and preserve them through a timing change.

## 3. `stagger()`

```js
delay: stagger(interval, {
  start,
  from,
  reversed,
  ease,
  grid,
  axis,
  modifier,
  use,
  total,
});
```

- Shared interface `interval` reads `delay-stagger` ([Use tokens and reduced motion with anime.js, “1. Reading and converting tokens”](tokens-reduced-motion.md#1-reading-and-converting-tokens)). An authored scene selects its interval or per-target offsets from its temporal treatment. A `[from, to]` range spreads across targets; `from` is `'first'`, `'center'`, `'last'`, `'random'` or an index; `grid: [cols, rows]` with `axis` distributes by position in a grid.
- Stagger values as well as delays: `scale: stagger([a, b])` spreads a value range.
- For a semantic sequence, choose an order that preserves meaning. A selected field or atmospheric composition may use a distribution rather than reading order; film randomness must reproduce under independent seeking ([Compose a sequence people can follow](../../films.md)).
- A stagger follows the reading or causal order of the content. Keep search results, lists, tables and keyboard targets available immediately; a decorative sequence must not delay them ([Show what happens first, “Cascades”](../temporal-hierarchy.md#cascades)).

## 4. Geometry built in the mount slot

If the renderer disallows SVG in markup or assets, create SVG in the mounted component. The still in the fallback is complete on its own; the island builds the SVG it animates with DOM APIs inside its mount slot, from the same sourced data the still shows ([Compose and check a live visual region](../../figures.md)).

```js
const NS = "http://www.w3.org/2000/svg";
const svgEl = document.createElementNS(NS, "svg");
svgEl.setAttribute("viewBox", viewBox);
const edge = svgEl.appendChild(document.createElementNS(NS, "path"));
edge.setAttribute("d", edgePath);
slot.append(svgEl);
```

- Set the start state (`utils.set`) on the built geometry before it becomes visible, then play; the fallback still stays until the built figure is ready to take its place.
- `svg.createDrawable` accepts paths, lines, polylines and rects and returns proxies whose `draw` value is two numbers from 0 to 1, the visible start and end of the stroke (`'0 1'` whole, `'1 1'` none). `vector-effect: non-scaling-stroke` makes drawing recompute every frame; avoid it.
- Draw a relation when both ends already exist; drawing is the relation arriving, not an ornament ([Compose and check a live visual region](../../figures.md)).

## 5. Drawing and morphing

```js
const [line] = svg.createDrawable(edge);
tl.add(line, { draw: ["0 0", "0 1"], duration: step }, "edge");
tl.add(shapeA, { d: svg.morphTo(shapeB), duration: step }, "result");
```

- `svg.morphTo(target, precision)` animates `d` on paths or `points` on polylines and polygons; source and target are the same element type, both built by the island. State what the morph means: identity-preserving change, conceptual conversion or editorial transition. Factual data views must not imply unsupported values; authored illustration need not depict one object persisting through every transformation.
- `svg.createMotionPath(path)` returns `translateX`, `translateY` and `rotate` tweens that follow a built path. A loop needs a selected temporal purpose and clock, not a fabricated live process ([Choose what advances the motion](../time-models.md)).

## 6. Detection

- Shared role positions or stagger intervals copied as literals; distinguish intentional authored scene timing.
- `'<'` read as "with the previous start" (it is the previous end; the start is `'<<'`).
- A timeline that must finish before the figure is readable or operable.
- A drawing, morph or path animation looping without a clock you chose ([Choose what advances the motion](../time-models.md)).
- Staggered reveal of every item in a collection.
- SVG markup or assets unsupported by the project’s renderer; an enhanced figure with no complete supported fallback.

## Sources

- [anime.js documentation: Timeline](https://animejs.com/documentation/timeline): timeline parameters and the `add`, `set`, `call`, `label`, `sync` and `remove` methods.
- [anime.js documentation: Time position](https://animejs.com/documentation/timeline/time-position): the exact meaning of every position syntax, including `'<'` versus `'<<'`.
- [anime.js documentation: Stagger](https://animejs.com/documentation/utilities/stagger): stagger values, ranges and every option.
- [anime.js documentation: createDrawable](https://animejs.com/documentation/svg/createdrawable): supported elements, the `draw` value syntax and the non-scaling-stroke cost.
- [anime.js documentation: morphTo](https://animejs.com/documentation/svg/morphto): the precision parameter and matching element types.
- [anime.js documentation: createMotionPath](https://animejs.com/documentation/svg/createmotionpath): the returned tween parameters and the offset.
- [MDN: Document.createElementNS()](https://developer.mozilla.org/en-US/docs/Web/API/Document/createElementNS): creating SVG elements in the SVG namespace at runtime.
