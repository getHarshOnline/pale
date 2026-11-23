+++
description = "Use it to build CSS transitions, the transform origin taken from the trigger and the reduced-motion form into a control."
uses = [{ scope = "local", resource_id = "animate/motion-implementation" }]
see_also = [{ scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/feedback-roles" }, { scope = "local", resource_id = "animate/motion-toolchain" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "animate/motion/token-presets" }, { scope = "local", resource_id = "animate/animejs/guide" }, { scope = "local", resource_id = "animate/time-models" }]

[guide]
exposure = "internal"
+++

# Build the origin and reduced form into the control

## 3. CSS first

- Standalone productive micro-states prefer CSS transitions reading token custom properties. Retain an engine already in use when the state belongs to its coordinated choreography. Required semantics and usable final states remain complete with scripts off.
- Springs in CSS: [Carry a design through the web build](../web.md).
- **Transform origin from the trigger.** Anything that belongs to a trigger (a popover, a menu, a preview) sets `transform-origin` from the trigger's position relative to the surface, so it grows from and returns to what opened it. A centred dialog keeps its centre. The rule is [Choose feedback roles and timing, “2. Rules”](feedback-roles.md#2-rules) (origin is truthful).
- **Per-frame writes go to the element, never to an inherited root property.** Writing a custom property on the root or a large container every frame restyles every descendant that inherits it. An island that drives a value per frame writes the moving element's own style, or registers the property with `@property` and `inherits: false` and sets it on that element.
- **`will-change` only while animating.** Set it when a motion starts on the element that moves and remove it when the motion ends, and only where a rendered defect (jitter, a one-pixel shift) was seen; a permanent `will-change` holds a compositor layer and its memory for nothing.
- Choose an engine for the component’s framework and job, with one writer per animated property (see [Choose the lightest animation mechanism](motion-toolchain.md)); interruption and suspension follow the clock you chose in [Choose what advances the motion](time-models.md). If the mount stays on the page, revert its motion and show the still. If it unmounts, also clear its listeners, observers and loops.

A dialog or popover mounted open may have no previous computed style. Check whether the supported browser gives it a starting style and allows its exit to finish before removal. If an animation replaces a complete `transform` property, keep the base position in the frames or separate layout and movement onto different elements. Check where the element sits during the transition as well as where it ends.

## 4. Reduced motion in code

For a theme that sets `data-motion`, read its stylesheet and token guide for the attribute values, when it restores a choice, and how it signals a change. The example below assumes a `motion-preference` event on changes. Other projects may listen to `matchMedia` or a component’s preference state.

- CSS: the reduced-motion selectors (`:root[data-motion="reduce"]` and the `prefers-reduced-motion` path) swap every motion custom property to its reduced form (`motion.reduced.<role>`, `motion.reduced.easing`, the world's `world.motion.reduced.*`). An immediate final state is valid where it communicates the same result; a blanket duration override alone cannot ensure that travel, hidden starts and cleanup are handled.
- Islands: check what the selected API handles automatically and what the component must implement. In this example an island reads `data-motion` when it mounts and reduces under the contract's reduce condition. The `motion-preference` event fires on a change only, never when a stored choice is restored at load, so the mount read is the only way to see a restored choice; the island listens for the event, reverts and takes its reduced form (designed in [Design the reduced-motion alternative](reduced-motion.md)); the code is in [Convert design tokens to Motion options, “3. Reduced motion”](motion/token-presets.md#3-reduced-motion), and [Build a page animation with anime.js](animejs/guide.md).

## Worked example

A popover grows from a trigger near the right edge. Resolve the trigger position relative to the moving drawing, keep the box in flow, and inspect the origin after the page scrolls. Switch reduced motion while opening and check that the same content and focus remain available in place.

## Sources

- [MDN: Using CSS custom properties](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Cascading_variables/Using_custom_properties): how the token build's custom properties inherit to the island's element on each world and mode scope.
- [MDN: Window.matchMedia](https://developer.mozilla.org/en-US/docs/Web/API/Window/matchMedia): reading `prefers-reduced-motion` from an island when `data-motion` is absent.
- [MDN: linear() easing function](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/easing-function/linear): how the token-emitted spring easing is expressed in CSS.
- [MDN: getComputedStyle](https://developer.mozilla.org/en-US/docs/Web/API/Window/getComputedStyle): reading the resolved custom properties from the island's own element.
- [Motion: CSS spring generation](https://motion.dev/docs/css): the `spring(visualDuration, bounce)` form whose string is a CSS duration plus `linear()` easing, used by the token build.
- [Motion: spring](https://motion.dev/docs/spring): spring options by visual duration and bounce or by stiffness, damping and mass, as the tokens express them.
- [Motion source: spring generator at a pinned commit](https://github.com/motiondivision/motion/blob/16c61222c4166bbef5ff6ab0051192faa79fa8c8/packages/motion-dom/src/animation/generators/spring.ts): `getSpringOptions`: a time-defined spring (`duration`, `visualDuration` or `bounce`, without physics parameters) sets its velocity to zero, an explicit `velocity` included.
- [Motion: web animation performance tier list](https://motion.dev/magazine/web-animation-performance-tier-list): per-frame writes to inherited custom properties as the costliest pattern, and the memory cost of promoted layers.
- [MDN: @property](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@property): registering a custom property with `inherits: false`.
- [MDN: will-change](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/will-change): why the hint is set just before a change and removed after, never left on at rest.
