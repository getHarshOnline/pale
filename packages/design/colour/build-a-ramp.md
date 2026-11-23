+++
description = "When the grounds, lines or reading ink need a coherent ladder of lightness."
uses = [{ scope = "local", resource_id = "colour" }]
see_also = [{ scope = "local", resource_id = "colour/contrast-wcag" }, { scope = "local", resource_id = "colour/map-gamut" }, { scope = "local", resource_id = "colour/meaning-and-context" }]

[guide]
exposure = "internal"
+++

# Build a perceptual ramp before assigning roles

Use this when the grounds, lines or reading ink need a coherent lightness ladder.

## Lightness carries the structure

- Decide the lightness ladder before choosing any hue. Paper, sheet, tint, line, strong-line, muted and ink are positions on one lightness axis; the gaps between them are what make text readable, rules visible and grounds distinct.
- Equal steps in OKLCH L aim for approximately equal perceived lightness steps. Inspect their actual appearance in context. Use a larger gap where the role needs it, especially between reading ink and ground.
- Contrast is a function of the luminance gap, so set the gap for the gate first (see [Measure contrast on the surface that people read](contrast-wcag.md)), then let hue and chroma vary within it.
- Test in greyscale. If two roles that must differ collapse to the same grey, the palette depends on hue for structure and fails for many readers (see [Make color one cue for a meaning](meaning-and-context.md)).

## Chroma is spent, not applied

- Grounds and ink carry low chroma: enough to give the stock and the ink a temperature, never so much that a page reads as tinted. Start with the temperature of its neutrals when the direction calls for quiet surfaces.
- High chroma is reserved for the few roles that must be found quickly: accent marks, signal (emission), and state. When everything is chromatic, nothing is findable.
- Spent is not withheld. A restrained palette still needs a recognizable character at rest. Suppress the accent during review. If two treatments differ only by a faint warm or cool tint, revisit their material and role relationships.
- The achievable chroma at a given lightness and hue is bounded by the gamut cusp. Pale and very dark steps hold little chroma; yellows peak light, blues peak dark. Set chroma per step against the sRGB boundary instead of one number for the whole ramp; a constant chroma request clips unevenly and makes some steps flat.
- Chroma can raise perceived brightness (the Helmholtz–Kohlrausch effect), so a vivid mark can look lighter than its measured L. Judge vivid roles on rendered pages, not in the numbers.

## Hue shifts along a ramp

- Real materials do not keep one hue from highlight to shadow. A ramp may rotate hue as it darkens or lightens (warmer highlights, deeper shadows, or the reverse) to stay alive instead of greying.
- Keep the rotation small and monotonic, and state the hue path when interpolating in OKLCH (shorter, longer, increasing, decreasing), so a derived value does not swing the long way round.
- Hue drift that is accidental (from sRGB clipping or HSL stepping) is not a hue shift; it is a defect. See [Map a color to its destination gamut](map-gamut.md).

## Worked example: find space for a raised panel

Start three warm grounds at L 0.96, 0.92 and 0.88. Add an ink that passes the reading criterion on the darkest ground. Build a line between two neighboring steps and check it only as a decorative separator if that is its job. A line marking a control needs the stronger criterion. Near the pale end, reduce chroma as needed for the destination. Read the same fixture in greyscale and with both modes before moving the ladder into tokens.

## Sources

- [Björn Ottosson: sRGB gamut clipping](https://bottosson.github.io/posts/gamutclipping/): the per-hue cusp of the sRGB gamut in OKLab, which explains why achievable chroma depends on lightness and hue.
- [Björn Ottosson: Okhsv and Okhsl](https://bottosson.github.io/posts/colorpicker/): why picker spaces trade orthogonal lightness against gamut-relative saturation, and what that means for choosing ramp steps.
- [CSS Color Module Level 4: hue interpolation](https://www.w3.org/TR/css-color-4/#hue-interpolation): the shorter, longer, increasing and decreasing hue paths when a ramp or derivation interpolates in OKLCH.
- [Radix Colors: understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale): a first-party example of assigning scale steps by job (grounds, component states, borders, solids, text) with light and dark scales paired.
