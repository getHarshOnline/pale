+++
description = "Use it to keep a colour's role when the output has less room for chroma, comparing the authored value with the token build and the rendered result."
uses = [{ scope = "local", resource_id = "colour" }]
see_also = [{ scope = "local", resource_id = "system" }, { scope = "local", resource_id = "colour/build-a-ramp" }]

[guide]
exposure = "internal"
+++

# Map a color to its destination gamut

Keep the color’s role when the output has less room for chroma. Compare authored coordinates with the token build and the rendered result.

## Gamut: sRGB, Display P3 and beyond

- **sRGB** is the usual web baseline and the space assumed by the WCAG formula when the rendering space is unknown. Every role must hold its meaning and contrast in sRGB.
- **Display P3** reaches noticeably further in greens, reds and oranges. It is an enhancement: a P3 value may deepen an emitter or a pigment-like ink on capable displays, but the sRGB rendering should already carry the full design.
- Wider spaces such as Rec. 2020 belong to delivery workflows that can check those outputs.

A colour's identity lives in its OKLCH coordinates, not in the gamut it happens to fit. Author the intent, then verify where it lands in each target gamut.

## Gamut mapping as CSS Color 4 defines it

- CSS maps an out-of-gamut colour to the destination by **constant-lightness, constant-hue chroma reduction in OKLCH**, with a relative-colorimetric intent: colours already inside the destination are unchanged. The specification allows three algorithms (binary search with local MINDE, EdgeSeeker, ray trace) with equivalent aims.
- The local-MINDE algorithm uses a ΔEOK threshold of 0.02 in OKLab for its local clipping decision. This threshold belongs to the algorithm.
- Naive per-channel clipping (what many JavaScript conversions do when they emit hex) shifts hue and lightness. Any tool that converts OKLCH to the stored sRGB value should compare a chroma-reduced result with direct clipping and use the conversion the delivery contract calls for.
- The DTCG Color module leaves gamut mapping to the translation tool, so a build step can silently choose clipping. Check the built value against the authored intent.

Consequence for design: when a ramp step asks for more chroma than sRGB holds at that lightness, the rendered colour loses chroma, not lightness. Two formerly distinct high-chroma roles can converge. Recheck legibility after the actual conversion. Decide chroma per step against the sRGB boundary rather than relying on mapping (see [Build a perceptual ramp before assigning roles](build-a-ramp.md)).

## Worked example: keep two status roles distinct in sRGB

Render two wide-gamut marks on their real surface and capture the same frame in the sRGB fallback. If both marks hit the same chroma limit, move one role’s lightness or hue within the target gamut, or add a shape or label. Inspect the built tokens for unintended clipping, then measure contrast on the rendered fallback. A display enhancement can preserve the more vivid pair.

## Sources

- [CSS Color Module Level 4](https://www.w3.org/TR/css-color-4/): the OKLab/OKLCH and Lab/LCH definitions, the HSL non-uniformity demonstrations, `none` components, interpolation spaces and section 14 (gamut mapping: chroma reduction in OKLCH, the three algorithms, the ΔEOK JND).
- [Björn Ottosson: sRGB gamut clipping](https://bottosson.github.io/posts/gamutclipping/): the shape of the sRGB gamut in OKLab (the per-hue cusp) and the compared clipping strategies.
- [MDN: oklch()](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/color_value/oklch): syntax, component ranges (chroma percentage reference), `none`, and browser support.
