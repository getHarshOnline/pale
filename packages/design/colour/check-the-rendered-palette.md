+++
description = "Use it when a palette misbehaves in the running interface, to pick the check that explains what you see and the change to make."
uses = [{ scope = "local", resource_id = "colour" }]

[guide]
exposure = "internal"
+++

# Diagnose a palette in the running interface

Use the failed state and surface to choose the check that can explain it.

| What you see | Check | Change if confirmed |
| --- | --- | --- |
| Three surfaces collapse into one | Capture the ladder in greyscale, then compare computed colors and built token values. | Widen a lightness step, or fix the token if output diverged from the intended value. |
| The middle of a gradient looks dull | Try the named interpolation space and hue route on the same stops. | Choose OKLab for a clean midpoint or a deliberate polar path. |
| The dark toggle works and system dark does not | Emulate system dark, then switch explicitly. Inspect the used scheme, form controls and scrollbars. | Route both paths through the same mode value. |
| Text fails only on a plate, animated field or tinted card | Sample the worst region and frame under the text. | Add a stable bed or change the material behind the text. |
| Two identities look alike | Suppress accent on the same fixture and compare complete treatments. | Revisit ground, ink, geometry, marks and material. |
| Focus or chart series disappear in forced colors | Emulate the mode, navigate by keyboard and inspect the real target setting when available. | Give the cue a label, geometry or system-color outline. Avoid broad opt-outs. |

## Worked example: find a false pass

The token checker passes metadata ink on the default ground. In the browser it fails over a tinted card after hover. Inspect the computed hover background and measure that pair. Change the tint or its ink role so all states pass. Re-run the keyboard and forced-colors check: a fix for the normal hover does not establish that the focus cue works.

## Sources

- [MDN: forced-colors](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/forced-colors): the properties forced to system colours and those forced to none, and the advice to make only small tweaks.
- [Microsoft Edge blog: Styling for Windows high contrast with forced colors](https://blogs.windows.com/msedgedev/2020/09/17/styling-for-windows-high-contrast-with-new-standards-for-forced-colors/): transparent outlines that appear in forced colours and system colour keywords.
- [Chrome DevTools: Emulate CSS media features](https://developer.chrome.com/docs/devtools/rendering/emulate-css): emulating colour scheme, forced colours, contrast preference and colour gamut.
- [Chrome DevTools: Apply effects, emulate vision deficiencies](https://developer.chrome.com/docs/devtools/rendering/apply-effects): achromatopsia, protanopia, deuteranopia and tritanopia emulation.
- [Chrome DevTools: Make your website more readable](https://developer.chrome.com/docs/devtools/accessibility/contrast): the colour picker's contrast readout on rendered text.
