+++
description = "Use it to deliver the designed face with its rights, glyphs and features, and to control the metrics of its fallback."
uses = [{ scope = "local", resource_id = "type" }]

[guide]
exposure = "internal"
+++

# Deliver the face and control fallback metrics

## Deliver the designed face and features

Keep role-to-family intent separate from font files and delivery mechanics. For each supported target establish its permitted source, rights, formats, scripts, glyphs, styles and required OpenType features. Subsetting that removes a required glyph or numeric feature can defeat an otherwise successful load. Variable fonts can reduce delivery duplication when their actual axes fit the needed treatments; their existence is not a reason to introduce every axis.

Inspect the font actually used on representative elements. A declared stack may silently resolve to a different family or synthetic weight/style. Keep real italic, small caps and numeric-feature evidence separate from requested settings. If the designed treatment is required and cannot be delivered, report that specific target gap or obtain an agreement on an alternative; do not relabel a fallback as acceptance.

Record the delivery decision in the target's font configuration:

| Decision | What must be known |
| --- | --- |
| Source and rights | The allowed local or remote asset route, or controlled installed face; embedding rights for that target. |
| Formats and subsets | Client formats, languages, scripts, glyphs and OpenType features the roles actually need. |
| Role mapping | Which shared family token each role consumes; no accidental inheritance or component-specific family substitution. |
| Fallback | Ordered alternatives and measured metrics, checked with the intended files blocked. |
| Loading | Which roles can block or swap, what the first render shows, and how a missing required face is detected. |

Optical size, grade and weight are different mechanisms. Verify a font has the axis, the renderer supports it and the consuming path applies it. CSS `font-optical-sizing` defaults to `auto` for a supporting face; explicitly setting `opsz` through `font-variation-settings` pins that axis rather than letting it follow the size. A canvas or transformed composition may need an explicit size-to-axis mapping when the displayed size differs from the size used to draw the text. Test the mapping and the face's range; a token named optical-size does not implement it. Keep an unimplemented option distinct from a defect in an already accepted treatment. [Hierarchy](hierarchy.md#choose-what-carries-each-level) explains the separate grade correction and its layout check.

## Match what is loaded to what can actually render

Inspect a representative element of every role in the browser's used-font inspector, including glyphs that might come from another face. A file in the network log or a computed family stack does not establish which glyphs rendered. Repeat for the supported platform families: one operating system's fallback is not evidence for another. Test the loaded design, delayed load and complete failure, and report where fallback replaces the intended face.

Choose the delivery route the target can support and that the rights permit. A ban on remote font requests can be met by allowed self-hosting; it does not itself require a platform-default design. Check current engine capabilities with their owner. If the target cannot load a face, try its allowed asset path or composing workspace. If the project uses a composed title image, keep equivalent text in the semantic document; native video follows its media-access contract. A face restriction can leave identity work open even while independent reading and interaction checks pass. An embedding restriction calls for an owner-approved designed alternative. Subset for the scripts and features used, and compare a variable file with the fixed faces the role actually needs. Prioritize fonts required for the first reading experience, and prevent an expressive opening from settling unnoticed on the wrong face.

## Check the actual face and its fallback

Inspect computed and rendered fonts before compensating for a wrong weight. Supply the real face and style, and use `font-synthesis` to prevent synthetic weight or style where the design requires that. Verify the resulting glyphs rather than assuming the declaration worked.

For fallback matching, distinguish the `size-adjust` descriptor on `@font-face` from the `font-size-adjust` property that preserves a chosen font metric across fallback faces. Derive adjustments from the intended face and inspect line breaks, clipping and target positions during delayed load and complete failure. For an x-height adjustment, measure x-height relative to font size. `from-font` reads the first available face; if that is a fallback, it cannot recover the missing designed face's aspect value. Leave an unmeasured explicit correction unset and record the missing measurement with the owning font configuration. A fallback correction preserves layout; it does not deliver an unavailable identity face.

## Worked example

Two tabular columns drift. Inspect the font actually used, then compare equal-length runs of 1s and 8s with the requested feature on and off. A subset may be missing the feature or falling back on one glyph. Block the font, compare the fallback’s line breaks and check the next load. If the target forbids custom font files, compare an available face with the intended character and seek agreement on that rendition. Size and metric overrides address fallback layout, while grade and optical size need real axes in the face.

## Sources for the decision

- [CSS Fonts](https://www.w3.org/TR/css-fonts-4/) and [MDN: font-face](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@font-face): the family, file, range and descriptor contract.
- [MDN: font display](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@font-face/font-display) and [Unicode ranges](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@font-face/unicode-range): loading periods and subset selection.
- [MDN: size-adjust](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@font-face/size-adjust) and [font-size-adjust](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/font-size-adjust): different controls for matching font metrics.
- [MDN: optical sizing](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/font-optical-sizing) and [font synthesis](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/font-synthesis): automatic axis behavior and synthetic styles.
- [MDN: variable fonts](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Fonts/Variable_fonts): registered and custom axes, ranges and delivery choices.
- [Firefox Fonts panel](https://firefox-source-docs.mozilla.org/devtools-user/page_inspector/how_to/edit_fonts/index.html): inspect used fonts and supported variable axes.
- [WOFF 2.0](https://www.w3.org/TR/WOFF2/): the compressed web-font format; confirm each target's accepted formats.
