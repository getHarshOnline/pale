+++
description = "Use it to pick the colour space for each operation, such as ramps, mixing, gamut checks and delivery."
uses = [{ scope = "local", resource_id = "colour" }]
see_also = [{ scope = "local", resource_id = "colour/contrast-wcag" }]

[guide]
exposure = "internal"
+++

# Choose a color space for the operation

Choose the representation for what you are doing. A color may pass through several spaces from design to delivery.

## Choose the space by the operation

| Operation | Space | Why |
| --- | --- | --- |
| Setting lightness steps, chroma and hue for a ramp or palette | OKLCH | Lightness, chroma and hue are close to independent; steps are approximately perceptually even |
| Mixing, blending, gradients, hover and pressed derivations | OKLab (or OKLCH with a stated hue path) | Straight lines stay clean; sRGB mixes darken and drift towards blue-grey mid-points |
| Contrast computation for the WCAG gate | relative luminance from sRGB | The gate is defined on it; see [Measure contrast on the surface that people read](contrast-wcag.md) |
| Physical light arithmetic (compositing, alpha, averaging pixels) | linear-light sRGB | Light adds linearly; perceptual spaces do not model it |
| Measuring a difference ("are these two roles distinguishable?") | distance in OKLab (ΔEOK) | Straight-line distance is a reasonable perceptual difference |
| Delivery to the page | the space the token build and delivery target can represent | Storage is an interchange decision, not a design decision |

HSL is a cylinder around sRGB, not a model of perception: two HSL colours at the same "lightness" can differ enormously in perceived lightness, and equal HSL hue steps are visually uneven (CSS Color 4 demonstrates both). Use it only where perception does not matter.

CIELAB and CIE LCH remain valid and are what many tools and the Material HCT model (via its tone axis) build on. OKLab corrects CIELAB's worst failure, hue drift in blues as chroma changes, and is the space CSS Color 4 uses for gamut mapping. Compare CIELAB-derived values after converting them, with their source white point, into the same comparison space. HCT takes hue and chroma from CAM16 and tone from CIELAB L*. Its tone number is not an OKLCH lightness value.

## OKLab and OKLCH: what the axes mean

- **L**: perceived lightness from black to diffuse white. The axis that carries legibility, hierarchy and the ramp's steps.
- **C**: chroma: colourfulness relative to a neutral of the same lightness. Zero is neutral. The achievable maximum depends on L, hue and the target gamut; it is not a fixed range.
- **h**: hue angle. OKLCH hue angles do not match HSL angles; never port an HSL hue number.
- **a / b**: the Cartesian form (green–red, blue–yellow opponent axes), which mixing uses.

Hue is undefined at zero chroma. A neutral stored with a hue invites a tint to appear when an interpolation or relative-colour edit adds chroma; neutrals carry `none` (CSS) or an explicitly missing hue component (DTCG) where the tool supports it.

## Worked example: choose the math for three jobs

| Job | Starting point | Decision |
| --- | --- | --- |
| Find three ground levels for a cool panel | OKLCH L values of 0.93, 0.90 and 0.87 | Hold hue near the material’s color and check each level’s usable chroma in sRGB. |
| Fade between two designed fills | Two authored colors | Try OKLab, then inspect the midpoint for an unwanted dip. Use a stated hue path if you need a polar mix. |
| Estimate the pixel beneath text on a translucent surface | The actual backdrop, fill and alpha | Use the renderer’s result for the contrast check. For a light calculation, work in linear light. |

If the palette arrives from an HCT tool at tone 60, ask the tool for the resulting display color and convert that result. Copying 60 into a percentage slot in OKLCH loses the different model and its gamut constraints.

## Sources

- [CSS Color Module Level 4](https://www.w3.org/TR/css-color-4/): the OKLab/OKLCH and Lab/LCH definitions, the HSL non-uniformity demonstrations, `none` components, interpolation spaces and section 14 (gamut mapping: chroma reduction in OKLCH, the three algorithms, the ΔEOK JND).
- [Björn Ottosson: A perceptual color space for image processing](https://bottosson.github.io/posts/oklab/): why OKLab was designed, its comparison against CIELAB, CIECAM02-UCS and IPT, and the blue hue-linearity fix.
- [Björn Ottosson: How software gets color wrong](https://bottosson.github.io/posts/colorwrong/): why blending in gamma-encoded sRGB darkens and shifts colours, and when linear light versus a perceptual space is the right tool.
- [Material Color Utilities](https://github.com/material-foundation/material-color-utilities): the HCT model (CAM16 hue and chroma with CIELAB lightness as tone) when a tool or partner palette arrives in HCT tones.
