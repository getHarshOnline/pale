+++
description = "Use it when the brief leaves an identity open, to find a physical or spatial idea in the subject and compare different ideas on the same content."
uses = [{ scope = "local", resource_id = "art-direction" }]
see_also = [{ scope = "local", resource_id = "art-direction/compare-identities" }]

[guide]
exposure = "internal"
+++

# Give an identity a subject and a working form

Start with what the work is for, who will use it, the content it carries and the qualities the brief rules out. Use references in their assigned roles. For an open direction, find a physical or spatial idea in the subject. A numbered stack of papers, a sequence of joined modules, or a margin of annotated drawings can organize more choices than a mood adjective. Compare genuinely different ideas on the same content with accent muted.

Read the accepted board, audience task, page types and sibling identities together. Inherit an approved identity when the task is a new page or shot within it. Where direction remains open, render two or three credible organizing ideas on a neutral fixture and use the agreed decision process to select one. Keep the selected direction and its reasons in the existing brief; put refused alternatives with the project's design decisions. A new scene or crop does not by itself require deriving another identity.

Name the main contrast and any supporting ones. A hard line inside a softly reflected surface can separate measurement from material. Write how the treatment behaves before naming values. A shared role can be deliberately neutral. Test the combination instead of counting how many roles differ from the neighboring identity.

Separate identity from its [expressive range](expressive-scale.md). A restrained reference page and an expansive introduction can carry the same identity at different intensities. State the required presence and permitted ceiling for each task. Two identities may share a range; the range does not tell them apart. Tie each recurring choice to the organizing idea or a controlled contrast, including why a dimension is deliberately neutral.

| What can give a work character | Where to look for it |
| --- | --- |
| Material and texture | Paper, metal, woven or glazed surfaces and the scale of grain. Keep running text legible. |
| Geometry and display voice | Edges, marks, silhouette, weight, width and useful display scale. Preserve the case of source identifiers. |
| Figure rhythm and crop | Alignment, caption placement, negative space, what a close view includes, and where the frame can cross a group. Never crop away the information the image carries. |
| Light and optical depth | Which things reflect, which communicate state through light, and how shadow, translucency or a focal plane locates them. |
| Expressive timing | How a surface settles, a field responds or a scene changes. Keep the learned response of shared controls. Give the reduced form the same intended state or relation. |

Keep shared focus, keyboard, hit areas, reading measure and semantics usable. Set local values with the people who maintain the theme or design tokens. Apply the decisions on the real pages. Follow the navigation into opened, generated and fallback views. A landing image can look distinctive while search, results and settings still use an unrelated treatment. Check the form in light, dark and reduced motion where supported. Use [Compare identities across the same design system](compare-identities.md) for relationships and shared behavior across identities.

## Carry the idea into the system that renders it

For each behavioral choice, describe the action before setting its parameters: how a field settles, how figures interrupt a reading sequence, which image information a crop protects, or how the reduced presentation communicates the same state. Find the existing recipe that does that job. A pattern name selects a recipe; its dimensions and timing still need real consumers. Reuse a suitable recipe across identities. If none fits, extend the owning vocabulary with its meaning and implementation together.

Keep the project brief's reasons and token references separate from the values maintained by the token system. Generate a joint comparison from those values when the project needs one; do not maintain a second handwritten matrix. A claim such as “the narrowest reading measure” belongs to a current comparison, since adding a sibling can invalidate it. The owner's statement about character or permitted ambiguity is a different kind of claim and stays with the brief.

Trace one choice all the way from its declared role through selection or resolution, generated styles, a component and the rendered page. A token called “grain” proves little if every component still paints a flat background. Check all the supported preference combinations and their actual entry paths. Deliberate inheritance is valid; a missing consumer or an override that erases the intended treatment is a defect. Inspect repeated page-level literals for a choice that belongs in the shared identity recipe.

Adding an identity also means updating the consumers that exist: its brief, token source and resolver context, build selection, theme assets and manifest, cross-reference marks and colors, and the relevant registries. Check marks in the selected font and browser; a valid font file does not establish that a particular glyph renders correctly or that its use is licensed. Rebuild using the repository's documented command and inspect pages and incoming references. Follow that repository's creation and publication permissions; a design comparison supplies neither.

## Worked example

A collection of field observations uses a ruled logbook as its organizing idea. Narrow rules separate actual records and their annotations. Dense date and unit columns remain legible. A detail view opens into the same reading plane rather than a generic tinted card. Compare it with a catalog that uses the same components. With accents muted, the sequence, typography and edges should still identify the field notes. If keyboard dismissal changes between the two, fix the shared control instead of styling around it.

## Sources

- [DTCG Resolver Module 2025.10](https://www.designtokens.org/tr/2025.10/resolver/): sets, modifiers, contexts and resolution order when the project's token system uses this model; verify the consuming implementation too.
- [Fluent 2 design tokens](https://fluent2.microsoft.design/design-tokens): global and alias tokens as a concrete example of keeping reusable values separate from their application to interface roles.
