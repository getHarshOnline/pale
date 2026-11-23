+++
description = "When two samples have similar numbers and look different, or a review mixes physical light with perceived colour."
uses = [{ scope = "local", resource_id = "colour" }]
see_also = [{ scope = "local", resource_id = "surfaces" }]

[guide]
exposure = "internal"
+++

# Tell lightness, brightness and chroma apart

Use this when two samples have similar numbers and look different, or when a review mixes physical light with perceived color.

## Terms that are not interchangeable

| Term | Means | Common mistake |
| --- | --- | --- |
| Lightness | perceived reflectance relative to a similarly lit white | treated as HSL L |
| Brightness | perceived intensity of light from a source | used for surfaces instead of emitters (see [Choose what material and light communicate](../surfaces.md)) |
| Chroma | colourfulness against a same-lightness neutral | treated as saturation |
| Saturation | colourfulness relative to the colour's own brightness | "same saturation" assumed to look equally vivid across hues |
| Luminance | physical, linear light (the Y of XYZ) | assumed to be perceptual; it is what WCAG ratios use |

## Worked example: separate a badge from its ground

A red badge and its gray ground use the same OKLCH lightness. The red appears brighter in the composition. Check relative luminance if the difference carries a label or a control, then remove hue and see what remains. If the badge marks a current state, add an outline, label or change in shape that survives the same-lightness comparison. Use OKLab distance to compare candidate swatches under the same viewing assumptions, then check the rendered result.

## Sources

- [CSS Color Module Level 4](https://www.w3.org/TR/css-color-4/): the OKLab/OKLCH and Lab/LCH definitions, the HSL non-uniformity demonstrations, `none` components, interpolation spaces and section 14 (gamut mapping: chroma reduction in OKLCH, the three algorithms, the ΔEOK JND).
- [Björn Ottosson: A perceptual color space for image processing](https://bottosson.github.io/posts/oklab/): why OKLab was designed, its comparison against CIELAB, CIECAM02-UCS and IPT, and the blue hue-linearity fix.
