+++
description = "Use it to find a slow or incorrect frame in the running interface and to check shared roles against the detection rules."
uses = [{ scope = "local", resource_id = "animate/motion-implementation" }]
see_also = [{ scope = "local", resource_id = "web" }]

[guide]
exposure = "internal"
+++

# Profile and repair motion in the browser

## 5. Detection rules

- A shared interface-role or world-character value authored as a literal instead of reading its token; an authored scene action is judged against the timing its action needs.
- Unexplained local arithmetic that changes a shared role (for example halving a stagger to feel quicker) instead of correcting its source.
- A shared control accidentally taking expressive timing, or an intended expressive moment flattened to ordinary feedback.
- Tokens read from the document root instead of the island's element, so the world scope is ignored; a literal substituted for an empty read.
- Layout-affecting properties used where composited motion expresses the same change. A selected layout morph preserves stable targets and requires measured performance.
- A spring outside the spring policy; CSS spring mistakes: [Carry a design through the web build](../web.md).
- A popover or menu scaling from its own centre; a custom property on the root written every frame; `will-change` left on at rest.
- A spring, role or world's expressive motion with no reduced form, or the reduced state implemented only through a global duration override that leaves travel, hidden starts or cleanup unresolved.
- An island engine loaded at a floating version.

## 6. Review questions

1. Classify values before auditing. A checker can flag literal shared-role times. When its rules allow scene-specific timing, mark the authored scene files and read the exclusions. Check that each excluded scene still follows its intended timing on the page.
2. Change a motion token and rebuild: did every stylesheet and island pick it up without edits?
3. With scripts off, is every productive state still correct in CSS?
4. In the reduced state, does each role and expressive moment show its designed form including a correct immediate state where appropriate?

## Worked example: fix a slow drag

| Before | Change | Check |
| --- | --- | --- |
| The pointer updates an inherited custom property on the document root on every frame. A trace shows descendants recalculating style. | Put the write on the moving element. If a custom property is useful, register it with `@property` and `inherits: false`. | Record the same drag on the same page and compare style work and dropped frames. |
| The drawer jumps a pixel when it leaves a compositing layer. | Set `will-change` on the drawer for that transition and clear it after it ends or is cancelled. | Slow both ends of the transition and check the resting frame. Check layer memory with several drawers or panels. |
| The drawer's server-rendered fallback and the hydrated component start at different transforms. | Use the same initial state in both. Start animation only after mount if the user has not asked for less motion. | Reload with scripts off and with an already stored reduced-motion choice. |

Keep the trace comparison and observed result in the component’s existing review or maintained performance check. A change to token plumbing justifies checking its consumers; a local visual repair does not require rebuilding unrelated motion.

## Sources

- [MDN: Using CSS custom properties](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Cascading_variables/Using_custom_properties): how the token build's custom properties inherit to the island's element on each world and mode scope.
- [MDN: Window.matchMedia](https://developer.mozilla.org/en-US/docs/Web/API/Window/matchMedia): reading `prefers-reduced-motion` from an island when `data-motion` is absent.
- [MDN: linear() easing function](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/easing-function/linear): how the token-emitted spring easing is expressed in CSS.
- [MDN: getComputedStyle](https://developer.mozilla.org/en-US/docs/Web/API/Window/getComputedStyle): reading the resolved custom properties from the island's own element.
- [Motion: CSS spring generation](https://motion.dev/docs/css): the `spring(visualDuration, bounce)` form whose string is a CSS duration plus `linear()` easing, used by the token build.
- [MDN: @property](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@property): registering a custom property with `inherits: false`.
- [MDN: will-change](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/will-change): why the hint is set just before a change and removed after, never left on at rest.
