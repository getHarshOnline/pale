+++
description = "When a floating layer must show enough of what passes behind it to keep depth or context."
uses = [{ scope = "local", resource_id = "web" }]
see_also = [{ scope = "local", resource_id = "surfaces/check-the-surface" }]
official_sources = [{ label = "Filter Effects Level 2", url = "https://drafts.csswg.org/filter-effects-2/" }]

[guide]
exposure = "internal"

[stack]
+++

# Render a floating layer with a legible fallback

Use this when a floating layer needs to show enough of what passes behind it to preserve depth or context. Choose a sufficient opaque fill first, then compare it with a filtered version. A backdrop filter samples the content within its backdrop root. Opacity, filters and masks on ancestors can create a different root than the one the design assumes. Inspect the layer stack before changing the blur.

The backdrop-root definition is still a draft. A transform or stacking context alone is not a backdrop root under that definition; do not remove transforms blindly when a blur appears empty. Inspect the ancestor's computed opacity, filters, masks, clipping and blending, then test the supported browsers.

## Build and test the layers separately

A useful stack has four parts: a legible opaque fallback bed; a supported backdrop blur or color adjustment; thin surface fills that establish density, lift or sheen; and an edge and cast shadow that locate it in the scene. Build the fallback first, then inspect the actual moving composite as translucency is introduced. The fallback contrast alone cannot establish the filtered version’s contrast.

If the chosen material needs refraction, define a displacement map, use its selected channels to displace the backdrop, and keep the strongest bend near the edge so the center remains readable. Optional dispersion separates and recombines color channels at slightly different displacement scales. A small blur and edge mask can settle that recombination. Check each primitive on the target: a support query for a surrounding property does not establish that the complete chain renders correctly.

| Condition | Useful fallback and check |
| --- | --- |
| Missing filter support | Use the plain bed, fills and edge; keep layout and hit regions unchanged. |
| Reduced transparency | Use an opaque surface without backdrop blur or refraction. |
| Forced colors | Use system colors and a visible border; check that filters do not obscure the result. |
| Increased contrast | Compare a matte surface that preserves text, focus and boundaries. |
| Reduced motion | Remove pointer-driven deformation while keeping the same control and state. |
| Missing script | Use the complete CSS surface when a generated map cannot be made. |

Measure blur radius, affected area and pass count during the actual movement on a representative phone as well as a desktop. Keep the filter area small and use `will-change` only where measured work justifies it, removing the hint after movement. Reuse a shared construction for repeated surfaces instead of copying chains and drifting values. Shared surface values come from the project's design roles; authored displacement-map geometry belongs to its mechanism. A pointer-deformed surface needs a reason in the selected treatment and a stable hit region. Put observations and corrections in the existing review.

| Detect | Inspect and repair |
| --- | --- |
| Contrast measured only on the opaque fallback | Check the text and controls over the worst moving composite as well. |
| A blur that shows nothing | Trace the sampling area and ancestor backdrop root; move or correct the responsible layer. |
| Refraction is the only complete version | Finish the plain stack first, then test the supported filter chain. |
| An unreadable or smeared center | Reduce central displacement, correct the mask or strengthen the fill. |
| Repeated components use different filter values | Share the construction and read its owning design roles. |
| Filters survive reduced transparency or forced colors | Exercise the appropriate opaque or system-color path. |
| The pointer bends the actual hit target | Keep the target stable; confine any justified deformation to the drawing and provide its reduced form. |
| A large or multipass surface misses its frame budget | Measure area, radius and passes; compare fewer passes or the complete plain treatment on the target phone. |

## Worked example: a narrow floating toolbar

```css
.toolbar {
  background: var(--toolbar-solid);
}
@supports (backdrop-filter: blur(1px)) {
  @media (prefers-reduced-transparency: no-preference) {
    .toolbar {
      background: var(--toolbar-translucent);
      backdrop-filter: blur(var(--toolbar-blur));
    }
  }
}
```

Keep the toolbar fully usable in the opaque version. Check the rendered text over the most limiting content and resize to a narrow view. Verify whether the stylesheet pipeline and target browser deliver the syntax and the preference query. Use a small affected area. Multiple filtered layers can increase cost and conceal depth. A more elaborate SVG refraction filter belongs only where the supported renderer, cost and visual result justify it. Recheck forced colors and the design’s scroll-edge treatment. See [Test translucency and texture where content moves](../surfaces/check-the-surface.md).

## Sources

- [Filter Effects Level 1](https://www.w3.org/TR/filter-effects-1/): filter primitives, their inputs, outputs and ordering.
- [Backdrop-root draft](https://drafts.csswg.org/filter-effects-2/#BackdropRoot): sampling boundaries and the current trigger definition; verify behavior on the target.
- [MDN `feDisplacementMap`](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element/feDisplacementMap) and [`feImage`](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element/feImage): selected map channels, displacement scale and image inputs.
- [MDN reduced transparency](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-transparency): the preference and its support; absence of the query does not establish a complete fallback.
- [MDN forced colors](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/forced-colors): system replacements and the surfaces that must remain visible.
