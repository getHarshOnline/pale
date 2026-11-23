+++
description = "To judge real text, controls and hit regions on the surface where people use them."
uses = [{ scope = "local", resource_id = "access/web" }]

[guide]
exposure = "internal"
+++

# Check rendered contrast and pointer targets

Use this to judge actual text, controls and hit regions. Choose the applicable criterion and the project's design target, then measure them separately. Inspect forced colors when a boundary or focus indicator relies on a fill or shadow.

Measure the rendered element rather than inferring a pass from its token pair. Check rest, hover, focus, pressed, selected, visited and error states where they apply, in each supported appearance and contrast or transparency preference. In the review, give the page, element, presentation state, width, measurement tool and result. Keep the applicable minimum and any stricter project target in separate columns. Compare the unrounded value with the threshold, even when the displayed result uses fewer decimal places.

## Apply the threshold to the element in use

| WCAG 2.2 check | Minimum | What to compare |
| --- | --- | --- |
| Text contrast, AA | 4.5:1 | Text or an image of text against its background |
| Large text contrast, AA | 3:1 | Text at least 18 pt (24 CSS px), or 14 pt bold (about 18.67 CSS px) |
| Text contrast, AAA | 7:1 | The enhanced text criterion, when adopted by the project |
| Large text contrast, AAA | 4.5:1 | The same large-text definition as AA, with the enhanced threshold |
| Non-text contrast, AA | 3:1 | A needed control, state or graphic against adjacent colors |
| Pointer target size, AA | 24 × 24 CSS px | The region that actually receives the input, or an applicable exception |
| Pointer target size, AAA | 44 × 44 CSS px | The enhanced criterion. Report a project target separately. |

A ratio of 4.49:1 does not pass a 4.5:1 threshold. Check the actual foreground and composite beneath it, including opacity, blend, background image and the brightest or darkest relevant patch beneath the glyphs. Use the defined foreground and rendered background colors for the ratio; anti-aliased edge pixels are not the text color. Thin strokes can still look faint after a nominal pass, so inspect legibility as well. Step through any moving backdrop during the reading interval. A sample at the center or an average of the background can miss the point that fails. Check the focus drawing against the surface it touches, and compare it in focused and unfocused states when assessing focus appearance. Inactive controls, decorative content, logos and incidental text have particular exceptions. Captions, placeholder text and metadata still need checking.

For the AAA focus-appearance criterion, compare captures of the same control focused and unfocused. An area at least as large as a 2 CSS px thick perimeter of the unfocused component must change by at least 3:1 at the same pixels. This is a separate comparison from AA contrast against adjacent colors. A 2 px inset ring can have too little area; measure its area rather than inferring a pass from line width. Check the criterion’s exceptions for unmodified or non-adjustable browser indicators.

Measure effective target width and height, including any padding or pseudo-element that accepts pointer input. When testing the spacing exception, center a 24 CSS px diameter circle on each undersized target's bounding box and check that it overlaps neither a neighboring target nor the equivalent circle of another undersized target. Other exceptions cover an equivalent control on the same page, inline text or line-height constraints, unmodified user agent sizing and essential presentation. Verify the reason for the exception. A 30 × 30 control can pass the AA size check and miss a 44 × 44 project target.

Inspect the actual receiving element, including overlapping layers. Sample the corners of the intended hit region with `document.elementFromPoint()` and exercise the control there; an invisible extension that another element intercepts is not usable target area.

Test error, current, visited and held states with color removed. Give chart series direct labels, markers or patterns. In running text, give a link a non-color cue. If relying on the sufficient technique that distinguishes it from surrounding text by contrast, use a 3:1 difference and add a non-color cue on hover and focus. Check the actual link against its background as well. A reveal needs a focus and touch route, and the end of a transition needs a readable state.

## Sources

- [Minimum text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html) and [enhanced text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-enhanced.html): text sizes, thresholds, exceptions and measurement without rounding up.
- [Non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html): needed control boundaries, states and graphics against adjacent colors.
- [Focus appearance](https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance.html): the enhanced area and same-pixel contrast checks, including browser-indicator exceptions.
- [Use of color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html): a second way to perceive states, categories and links.
- [Minimum pointer target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html) and [enhanced pointer target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-enhanced.html): effective hit regions, spacing and exceptions, with the enhanced size kept distinct.

## Worked example

Two 20 × 20 CSS px controls have centers 22 px apart. Their 24 px circles overlap, so they fail the spacing exception. Increase the separation or enlarge their effective hit regions. Repeat after the layout compresses. Report both the applicable criterion and the project's own target.

Check focus with the pointer resting on the control, in both modes and in forced colors. Compare the measurement with the observed cue, not a variable pair from the palette.
