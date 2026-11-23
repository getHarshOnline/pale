+++
description = "When colour marks a link, series, warning, category or product state and needs a second cue."
uses = [{ scope = "local", resource_id = "colour" }]

[guide]
exposure = "internal"
+++

# Make color one cue for a meaning

Use a second cue when color marks a link, series, warning, category or product state.

## Match the cue to the job

The job travels across products even when the hue changes. A relation can use an accent, a running process can use a steady signal, and an error can use a message and icon. Keep these roles independent even if two current values happen to be equal. A red or vermilion failure color is one possible convention. Check the product’s actual language, cultural context and relation to its other marks. Give errors text that can be found and read. Do not make an operational signal pulsing by default.

A link into a different publication can use a small accent from its destination alongside its mark and underline. Keep the surrounding text readable on the current surface. An identity needs more than the accent. Compare structure, material and type with color removed. Check color vision deficiencies, greyscale and forced colors in the rendered page. Simulation helps find a defect. It does not replace accessible names, labels, marks or real user checks.

## Worked example: distinguish two chart series

A red line and a green line have similar luminance. Assign a solid line with circles and a dashed line with squares, label the series directly, then view the actual chart in greyscale and emulated color-vision deficiencies. In forced colors, confirm the markers and labels still distinguish the series. Put the same cue in the legend if it remains.

## Sources

- [WCAG 2.2 Understanding: Use of Color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html): when colour is the only visual means, and when a lightness difference counts as an additional cue for links.
- [WCAG 2.2 Understanding: Non-text Contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html): the contrast owed by state indicators and graphical objects that carry meaning.
- [Machado, Oliveira and Fernandes: A physiologically-based model for simulation of color vision deficiency](https://www.inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html): the simulation model and matrices used by current deficiency simulators.
- [National Eye Institute: Color blindness](https://www.nei.nih.gov/eye-health-information/eye-conditions-and-diseases/color-blindness): prevalence and the most common (red–green) deficiency.
- [CSS Color Adjustment Module Level 1](https://www.w3.org/TR/css-color-adjust-1/): what forced colours replace and what authors must keep so meaning survives.
