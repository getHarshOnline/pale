+++
description = "Use it to read shared interface timing and expressive character from the project's design tokens and to choose a spring or a tween."
uses = [{ scope = "local", resource_id = "animate/motion-implementation" }]
see_also = [{ scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "animate/feedback-roles" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "animate/motion/guide" }, { scope = "local", resource_id = "animate/animejs/guide" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/interruption-reversal" }]

[guide]
exposure = "internal"
+++

# Read timing and spring roles from the system

## 1. Which values come from tokens

Read shared interface timing and expressive character from the project’s design tokens. The names in this table illustrate one token scheme. Authored scene actions retain their specified temporal treatment, including when delivered inside a page ([Compose and check a live visual region](../figures.md)). Classify the value's role before requiring a token. Scene timing does not redefine a productive control role.

| Token group | Carries | Used by |
| --- | --- | --- |
| `duration.*`, `delay.*` and `easing.productive` | Productive tween roles by id (feedback, preview, enter, leave, result, sequence; the roles are [Choose feedback roles and timing, “1. The role contract”](feedback-roles.md#1-the-role-contract)), the preview intent delay, the productive stagger step `delay.stagger` and the one productive easing family | Every productive role |
| `motion.spring.productive.*` | The productive spring as visual duration and bounce | Productive roles that settle physically (a highlight, a drawer) |
| `duration.expressive` with `world.motion.expressive-scale`, `world.motion.expressive-easing`, `world.motion.expressive-spring.*` | The world's expressive character: pace and curve family; an expressive stagger is `delay.stagger` scaled by `world.motion.expressive-scale` | The material event and islands only |
| `world.motion.decay` | How state-bearing light lets go | Emission decay (see [Choose what material and light communicate](../surfaces.md)) |
| `motion.reduced.<role>` and `motion.reduced.easing`; `world.motion.reduced.expressive-scale`, `world.motion.reduced.expressive-easing`, `world.motion.reduced.decay` | Each role's reduced form and the one reduced curve; each world's expressive character and decay without travel | Every role and every expressive moment in the reduced state |

In this example, shared controls keep their learned timing across identities while expressive moments use each world’s character. Follow the actual project’s roles; avoid changing a shared control’s response merely to make one page feel different.

## 2. How islands take their values and the spring policy

Use the project’s token output and spring policy. The CSS-based example below shows how one system keeps stylesheet and script behavior aligned. [Build a page animation with Motion](motion/guide.md) and [Build a page animation with anime.js](animejs/guide.md) hold only their library's conversion code.

- In this example, the token build emits motion tokens as CSS custom properties on the world and mode scope, and each spring also as a duration plus a `linear()` easing.
- Read tokens from the island's own element at runtime (computed style), so the world, colour palette and mode in force at that point of the page are the ones it uses. There is no separate preset module to keep in step with the tokens.
- An empty or invalid read is a missing value in this scope. Fix its source or explain the gap in the existing component review; retain the usable still rather than feeding an invalid value to the engine.
- Convert units at the library boundary. Keep intended compositions visible: for example, this scheme multiplies expressive duration or stagger by the world’s expressive scale, and reduced expressive duration by its reduced scale. Shared controls use their role values as read. A further multiplier is a design choice to put in the shared system or the authored scene, not an unexplained local correction.

**Example spring policy for islands.**

| Where | Spring source |
| --- | --- |
| Controls and shells (CSS) | The token-emitted `linear()` easing over a productive tween ([Carry a design through the web build](../web.md)) |
| A Motion island | Springs animated from the token parameters `visual-duration` and `bounce` |
| An anime.js island | The token-emitted `linear()` easing string or the token cubic-béziers; never its own spring |

Motion’s duration-based spring in the pinned source below discards initial velocity. A released gesture that must keep momentum therefore needs a supported physics model and shared parameters for that task. If the project has not defined them, put that choice with the system owner; do not silently substitute a no-momentum settle or copy arbitrary stiffness, damping and mass from a demo. Another project may deliberately define physics-based springs. Verify its release in the rendered component for both position and velocity continuity.

Check for accidental library defaults and values that bypass the project’s chosen policy. A sampled easing can reproduce a predetermined spring curve, but it cannot inherit an arbitrary release velocity by itself. Keep the implementation in the component and the observed result in its maintained test or existing review.

## Worked example

A token build emits a spring duration and a sampled easing. Read both values and convert milliseconds once at the animation boundary. Compare the CSS treatment with the island at the same elapsed time. If a gesture needs its release velocity, verify that behavior separately before substituting a sampled easing.

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
