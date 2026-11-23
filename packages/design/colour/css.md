+++
description = "When implementing colour roles, state derivations or light and dark pairs in a stylesheet."
uses = [{ scope = "local", resource_id = "colour" }]
see_also = [{ scope = "local", resource_id = "colour/map-gamut" }, { scope = "local", resource_id = "colour/map-ramp-stops-to-roles" }, { scope = "local", resource_id = "surfaces" }]
official_sources = [{ label = "CSS Color Module Level 4", url = "https://www.w3.org/TR/css-color-4/" }, { label = "CSS Color Module Level 5", url = "https://www.w3.org/TR/css-color-5/" }]

[guide]
exposure = "internal"

[stack]
+++

# Apply color roles in CSS

Use this overlay when implementing color roles, state derivations or paired modes in a stylesheet.

## Perceptual colour: oklch() and oklab()

- `oklch(L C H / A)`: lightness as a number from 0 to 1 or a percentage; chroma from 0 upward, where a percentage refers to 0.4 (not 1); hue in degrees. Any component may be `none`.
- A browser maps an out-of-gamut value to its destination. Compare the result in the target gamut and see [Map a color to its destination gamut](map-gamut.md).
- `oklab(L a b / A)` is the Cartesian form; use it when mixing, not when reasoning about a ramp.

## Mixing: color-mix()

- `color-mix(in <space>, <colour> <p>?, <colour> <p>?)`. The interpolation space is optional and defaults to OKLab with the shorter hue path. State it anyway so the intent is visible.
- Percentages normalise to 100%; if they sum to less, the result gains transparency.
- It is the syntax for state variants that move a role towards ink or paper (hover, pressed, subtle grounds); the amount comes from a `color.state.*` fraction, as `color-mix(in oklab, var(--color-sheet), var(--color-ink) calc(var(--color-state-hover) * 100%))` ([Assign ramp stops and derive control states](map-ramp-stops-to-roles.md)). In a polar space, name the hue path when the two colours are far apart in hue.

## Deriving: relative colour syntax

- `oklch(from <origin> l c h)` destructures an origin colour into channels in the output function's space; each channel may be replaced or computed with `calc()`. Channel keywords resolve to plain numbers, so arithmetic uses unitless values (`calc(l - …)`, not a percentage).
- It derives a state from its base role while holding the other channels: lower `l` for a pressed ground, reduce `c` for a quiet tint, adjust `alpha` for a translucent layer; use a shared derivation value when several controls need the same relationship.
- It is the newest part of the syntax. Guard it with `@supports` using a literal colour (custom properties are unreliable inside the test) and ship a complete fallback: make the unenhanced page work first.

## Gradients and interpolation

- Gradients accept an interpolation method (`linear-gradient(in oklch, …)`). Interpolate in OKLab for clean transitions; in OKLCH, state the hue path.
- Choose a gradient for its role in the composition: material, illumination, atmosphere or graphic transition. [Choose what material and light communicate](../surfaces.md) separates scene light from status. Check whether a gradient weakens reading or replaces a material the reference calls for.

## Wide gamut

- `color(display-p3 r g b)` and wide-chroma `oklch()` values reach beyond sRGB on capable displays. Query capability with `@media (color-gamut: p3)` and provide the P3 value as an enhancement over a complete sRGB design.
- The WCAG gate is judged on the sRGB rendering.

## Pair modes and leave a fallback

`light-dark()` follows the element’s used color scheme. Allow the schemes the interface supports and set the active scheme for an explicit mode so controls and scrollbars agree with the page. Check the system route as well. Use role values for `::selection`, `caret-color` and `accent-color` if those surfaces are part of the design. An automatic maximum-contrast foreground such as `contrast-color()` cannot decide what the role should look like.

## Worked example: derive a panel’s pressed state

```css
:root {
  color-scheme: light dark;
}
[data-mode="dark"] {
  color-scheme: dark;
}
[data-mode="light"] {
  color-scheme: light;
}

.panel {
  background: var(--panel);
  color: var(--ink);
}

@supports (color: color-mix(in oklab, white, black)) {
  .panel:active {
    background: color-mix(
      in oklab,
      var(--panel),
      var(--ink) calc(var(--pressed-fraction) * 100%)
    );
  }
}
```

The component’s token system supplies the pair and the mode’s fraction. Check the rendered pressed pair and confirm the panel, text and focus cue work where the enhanced syntax is unavailable.

## Sources

- [CSS Color Module Level 5](https://www.w3.org/TR/css-color-5/): the normative definitions of `color-mix()`, relative colour syntax, `light-dark()` and `contrast-color()`.
- [MDN: color-mix()](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/color_value/color-mix): the optional interpolation method (default OKLab, shorter hue), percentage normalisation and support.
- [MDN: Using relative colors](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Colors/Using_relative_colors): channel keywords per function, unitless channel arithmetic, output space and the `@supports` caveat with custom properties.
- [MDN: light-dark()](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/color_value/light-dark): the colour-scheme requirement, accepted argument types and support.
- [MDN: oklch()](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/color_value/oklch): component ranges, the chroma percentage reference and `none`.
- [CSS Color Module Level 4: interpolation](https://www.w3.org/TR/css-color-4/#interpolation): interpolation spaces and hue paths used by gradients and mixing.
