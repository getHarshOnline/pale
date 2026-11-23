+++
description = "Use it to design the same colour roles for a dark room, choosing the ground, reading ink and key light together."
uses = [{ scope = "local", resource_id = "colour" }]
see_also = [{ scope = "local", resource_id = "assets" }, { scope = "local", resource_id = "surfaces" }]

[guide]
exposure = "internal"
+++

# Design the same roles for a dark room

Compare each light and dark role side by side. Keep the reading order, material and state cues recognizable in both.

## Decide the dim-room treatment

An inversion flips the ramp while keeping its old chroma. Quiet color can glare, a warm stock can go slate, and every bright region can look lit. Choose the ground, reading ink and key light together. Give raised surfaces room above the ground. Pure black and white are valid for a chosen treatment, but examine glyph halation, lost depth and the contrast needed by thin type. For a reading surface, a near-neutral ink often carries more of the source character than white.

Use smaller chroma on large areas when the same hue reads too loudly in dark. Keep enough chroma and lightness on small marks to recognize their role. A dark-first design can have a light counterpart with the same temperature and a signal in the same place. Give scene illumination, relation accents and operational state different jobs. Emission reads naturally in dark, so ration it where an active state needs to stand out.

## Test the pair and the route into it

Compare both modes on the same content. Set the dark-mode state fractions separately and check each derived pair. Try a system preference change and the explicit mode control. They should reach the same dark surface when they ask for the same mode. Inspect form controls, scrollbars, text selection and focus. Compare whole page treatments with accents suppressed. Shared swatches can be right when type, geometry, material and light still carry the intended differences.

A host mode does not regrade its documentary photograph or encoded film. Use a separate image variant only when it is part of the asset design. See [Make an asset that belongs in its destination](../assets.md) and [Choose what material and light communicate](../surfaces.md).

## Derive each role in the dim room

Decide dark ground, ink and the light that surrounds them together. A near-black floor can leave little space for depth steps. Pure white text can flare on it. Try a dark ground with room for raised surfaces, then a light, low-chroma reading ink whose temperature belongs to the material. Use small perceptual-lightness steps for depth. Reduce chroma on large dark fields and keep useful chroma in smaller marks. An accent may need a lighter value in dark to keep its character and remain legible.

Read hover, press and held values on their dark ground. A fixed mixing fraction can look much weaker there. Use the actual rest and state combinations to decide the mix and measure each against adjacent text and marks. Keep the state signal distinct from the accent that marks navigation or relation. Review the complete dark page with accent subdued. If type, geometry, material and composition disappear into the same gray treatment, the palette comparison has missed the main issue. Preserve a supplied photograph or film's intended grade while deriving the surrounding plate and label treatment.

## Worked example: dim a printed index

The light index reads as warm stock and dense ink. In dark, test a warm low-chroma ground, a slightly lighter raised card, a light ink with the same temperature and a small high-chroma status mark. Compare the body copy, fine metadata, links and every card state on both grounds. If the mark looks like another accent or the two grounds collapse with accent hidden, change their relationships before adopting the pair.

## Sources

- [Material Design 2: Dark theme](https://m2.material.io/design/color/dark-theme.html): a first-party rationale for dark-grey rather than black grounds, lighter raised surfaces, and desaturated colour on dark.
- [Apple Human Interface Guidelines: Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode): base versus elevated backgrounds and checking contrast in both appearances.
- [WCAG 2.2 Understanding: Contrast (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html): why each mode's pairs are measured on their own and for exceeding the minimum with thin faces.
- [Radix Colors: understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale): an example of a dark scale designed as its own scale and paired by step with the light one.
