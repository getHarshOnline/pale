+++
description = "Use it to build hierarchy from the relationships between a few distinguishable levels, with measure and rhythm."
uses = [{ scope = "local", resource_id = "type" }]

[guide]
exposure = "internal"
+++

# Set type hierarchy, measure and rhythm

## Build hierarchy with relationships

Use the established scale and role tokens. Diagnose an unclear level through size, weight, face, case and spacing before adding more variables. Keep few distinguishable steps. Two visually indistinguishable levels may need merging or a meaningful second distinction. A heading already expressed through size, weight, color and tracking may need fewer competing signals. If a needed size falls outside the scale, choose an appropriate existing step or deliberately extend the shared scale and check its consumers.

Address crowding through grouping, order, measure and disclosure before shrinking reading text. Keep qualifiers and necessary metadata legible. Put headings closer to what they introduce, connect captions to figures and distinguish label/value spacing from the space between unrelated groups. A dense page needs a clear first read as well as accessible detail.

Judge tracking and leading for the delivered face, size, script and line length. Large Latin display may benefit from different spacing than small capitals; those adjustments are not rules for every script. Shared roles should resolve consistently unless the actual content or selected context warrants a change. Do not force unequal settings or a golden ratio without a rendered reason. Inspect paragraph rhythm and real line lengths on narrow and wide layouts.

## Choose what carries each level

Give each weight and size a job in the hierarchy. Weight can distinguish small labels where size differences are hard to see. An editorial opening or expressive peak can need a larger display-to-heading gap than a reading or decision page; choose from the brief rather than applying one dramatic scale everywhere. Use the reading face’s own italic or weight for emphasis.

Leading belongs to the role and size step; a paragraph needs enough to track the next line. Large, short display lines can often sit closer than sustained reading, but their shapes and script must fit. Relate block spacing to that rhythm. Choose paragraph spacing or first-line indentation for a passage; do not double-mark the same paragraph boundary without a reason. Check paragraphs of the same role for accidental differences in leading.

Under dark conditions, inspect each voice and its fine strokes at native size; a passing contrast ratio does not establish equivalent optical weight. A supported `GRAD` axis can adjust weight while preserving widths. Verify that the consuming renderer applies it and that line breaks remain stable. An ordinary weight step that reflows the page is not an equivalent correction. Keep a missing or unmeasured grade setting open with the owning system instead of introducing an unexplained component override.

## Separate the outline from the scale

Choose the heading level by the document outline and adjust its appearance through the role. A smaller-looking section heading keeps its semantic level. The scale controls appearance; it does not authorize skipping an outline level.

Bound prose by the system's reading measure rather than a percentage that grows indefinitely with the viewport. Count characters on several full rendered lines at wide and narrow widths, using the actual script and face. Display lines follow sense rather than the prose measure. A narrow handheld column needs its own composition while preserving the reading role's legible size, leading and ink. Keep metadata short enough to scan, and move it or regroup it when crowded. In a table, remove an unnecessary column, move details to a disclosure or choose a different form before making the figures harder to read; preserve access to the necessary values and qualifications.

## Worked example

A heading and metadata line look equally important. Keep the semantic headings in order and tune size and spacing first. Keep the heading closer to the first paragraph than to the preceding section. Try a reading measure around 60 characters as a starting point for Latin body copy, then check the actual face and script. Increase room between sections without weakening the bond between a label and its value. Check the result at narrow and wide widths.

## Sources for the decision

- [Practical Typography: line length](https://practicaltypography.com/line-length.html), [line spacing](https://practicaltypography.com/line-spacing.html) and [headings](https://practicaltypography.com/headings.html): judge measure, leading and proximity together.
- [The Elements of Typographic Style Applied to the Web](https://webtypography.net/toc): worked relationships between scale, paragraph rhythm and CSS.
- [Material type-scale tokens](https://m3.material.io/styles/typography/type-scale-tokens): one implementation of named role and scale relationships, not a required scale for other systems.
- [Apple: The details of UI typography](https://developer.apple.com/videos/play/wwdc2020/10175/): size-specific tracking and optical-size behavior; this does not establish a universal leading rule.
- [OpenType grade-axis proposal](https://github.com/TypeNetwork/Opentype-1.8-Axis-Proposal/blob/gh-pages/proposal/Grade/ProposalSummary.md): the intended distinction between grade and width-changing weight, subject to the actual face and renderer.
