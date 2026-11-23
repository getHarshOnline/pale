+++
description = "Use it to measure the WCAG contrast ratio on the surface people read, starting from the weakest text or control pair."
uses = [{ scope = "local", resource_id = "colour" }]
see_also = [{ scope = "local", resource_id = "type" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "colour/design-a-dark-palette" }]

[guide]
exposure = "internal"
+++

# Measure contrast on the surface that people read

Find the weakest text or control pair on the actual surface. Apply the criterion for the type of content and state.

## What the WCAG ratio measures

- The ratio compares the **relative luminance** of two colours (the normative formula is in the Sources); it is a luminance gap, blind to hue and chroma.
- It is computed in sRGB unless another colour space is known to process and display the content. A P3 value is judged as it renders in sRGB for the gate.
- The text thresholds compensate for the contrast-sensitivity loss typical of roughly 20/40 vision. They are floors, not targets for comfortable long reading.
- The ratio is symmetric: it does not know which colour is text and which is ground.

## Where the ratio misleads

- **Polarity.** Light text on a dark ground and dark text on a light ground with the same ratio do not read the same. Light-on-dark text of small size and thin weight tends to read weaker than its ratio suggests, and dark palettes near black compress luminance steps that the ratio still counts as sufficient. Dark palettes therefore need margin above the gate for reading text, not the bare minimum (see [Design the same roles for a dark room](design-a-dark-palette.md)).
- **Spatial frequency.** Thin strokes, small sizes and light weights need more contrast than the size-binary WCAG rule implies; anti-aliasing renders thin text fainter than its declared colour. WCAG itself advises exceeding the minimum for thin or unusual faces. Coordinate with [Make typography carry the intended meaning](../type.md) rather than lowering weight and keeping the colour.
- **Hue.** Luminance ignores chroma, so two vivid colours with a passing ratio can still vibrate or blur at their edge; and a chromatic accent may look more distinct than its ratio claims (chroma adds perceived lightness). Judge vivid pairs on the page.
- **Adjacency.** Non-text contrast is against **adjacent** colours: a focus ring is measured against both the control and the surface around it, and a chart series against its neighbours.

## Composited surfaces

A token pair gives you the intended colors. Look at the rendered background behind the glyph. Contrast changes whenever the ground is not a flat, opaque colour:

- translucent surfaces and alpha fills take their colour from whatever lies beneath;
- stock texture, raster and grain raise and lower the local ground under text;
- emission and halation brighten the ground around an emitter ([Choose what material and light communicate](../surfaces.md));
- images, plates and visual islands vary point to point ([Compose and check a live visual region](../figures.md)).

For each, the worst point under the glyphs governs; over a moving or scrubbed ground, it is the worst point in any frame the text sits over while it is read. When material would push text below its floor, the material yields: reduce or remove the texture under the text run, or give the text an opaque bed. Legibility never pays for material.

## Designing for the gate

1. Set the lightness ladder so that ink and muted clear the text threshold on **every** ground they may sit on, including cards, tints and overlays.
2. Give accent the text threshold on the grounds where it marks links, and the critical-state ink the text threshold wherever it colors a message. Measure required control indicators and focus against their relevant adjacent surfaces.
3. Leave margin above the gate in dark modes and for small or light-weight system text.
4. Check derived states (hover, pressed, selected, visited) as pairs of their own; a derivation that darkens a ground can break the text on it.
5. Let the checker enforce the token-level floors, then measure rendered elements ([Make the browser task accessible](../access/web.md)); the token check catches a different class of error from the page check.

## Detections

| Detect | Failure | Do |
| --- | --- | --- |
| Contrast table built only from token pairs | Intent reported as fact | Measure rendered elements ([Make the browser task accessible](../access/web.md)) |
| Dark-mode reading text at the bare threshold looks weak | Polarity and spatial frequency | Add margin; raise the ink's lightness |
| Muted metadata passes on paper, fails on tint or a plate | Only one ground checked | Check every ground the role can sit on |
| Text over texture or emission passes at its darkest point only | Average or best point measured | Worst point governs; the material yields |
| An APCA score cited as a pass | Research metric used as conformance | Report the WCAG ratio; APCA only to choose among passing pairs |
| Thin display or system text in a pale colour | Weight lowered, colour kept | Raise weight or contrast together ([Make typography carry the intended meaning](../type.md)) |

## Worked example: small text on a glass card

Measure a translucent card on its light ground, tinted state and brightest image frame. Suppose the first two text pairs measure 5.1:1 and 4.8:1, and the image makes one region 4.2:1. For ordinary text that region fails the 4.5:1 minimum. Add a stable text surface or reduce the translucency, then measure all three again. Keep a margin for thin letterforms and check the neighboring colors around the focus ring separately.

## Sources

- [WCAG 2.2 Understanding: Contrast (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html): the rationale behind 4.5 : 1 and 3 : 1 (contrast sensitivity at about 20/40 vision), the sRGB assumption, no rounding, and the advice about thin or unusual fonts.
- [WCAG 2.2 Understanding: Non-text Contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html): what "adjacent colours" means for components, states, focus indicators and chart segments.
- [WCAG 2.2: relative luminance and contrast ratio definitions](https://www.w3.org/TR/WCAG22/#dfn-relative-luminance): the normative luminance formula and the contrast-ratio definition.
- [Björn Ottosson: How software gets color wrong](https://bottosson.github.io/posts/colorwrong/): why linear light matters when compositing translucent layers before measuring the ground.
