+++
description = "When related products, publications or brands need a recognisable identity of their own while sharing interface behaviour."
uses = [{ scope = "local", resource_id = "art-direction" }]

[guide]
exposure = "internal"
+++

# Compare identities across the same design system

Use this when several related products, publications or brands need a recognizable identity while sharing interface behavior. Start with what each subject does, how it should feel, and the reference that establishes its direction. Judge a whole treatment. A count of changed values cannot tell you whether a reader can distinguish two products.

| Decision | What to vary | What to preserve |
| --- | --- | --- |
| Material and raster | Tint, temperature, grain, repetition and scale | Reading contrast and the job of the surface |
| Geometry and marks | Edges, rules, crop and shape language | Hit areas, focus and identifiable content |
| Display voice | Face, width, weight or another real axis | Legible reading and source case for literal strings |
| Figure rhythm | Interval, scale, alignment and caption placement | Units, sources and reading order |
| Light and depth | Key, shadow, raised layers, glow and falloff | The difference between a state and a decorative accent |
| Timing and cuts | Character of an expressive transition, hold or edit | Control response and the meaningful reduced form |

Compare each at rest, during its main action and in the available appearances. Compare identity with the accent suppressed. A shared pattern may work in two products when it serves the same job. Keep actual pattern names and permission for crops or bleeds in the project’s design system. A named crop can allow a passage only where the composition also has room for it.

Read the local bounds alongside each choice. A system may require one key-light direction, one translucent layer, a fixed set of state-bearing emitters or a nonzero emission floor. Preserve those requirements where they govern the work; they are not universal rules for every interface. Likewise, a crop permission and the page's expressive allowance both have to admit a bleed. A broad permission in one does not override a restriction in the other. Keep the actual scale and pattern vocabulary in their owning system.

Compare the same shared control across identities in the browser: its rendered size and hit area, focus indicator, keyboard sequence, dismissal and feedback timing. Computed styles can expose a changed outline or transition duration; use the control too, since matching values do not prove matching behavior. Values, units, qualifications, sources and status remain the same across treatments. An identity cannot turn a measurement into rounded prose or notation that changes what the source states.

## Make the distinctness comparison able to fail

When distinction is a requirement, use equivalent content, framing and states in actual browser captures. For a system supporting light and dark with full and reduced motion, compare all four combinations, including explicit and system-resolved paths where both exist. Use the project's real set of page types and contexts:

| Surface | What the comparison should expose |
| --- | --- |
| Opening or home | The organizing idea reaches the reader without relying on an explanatory slogan. |
| Quiet reading or reference | Material, type and figure rhythm still identify the work at its lowest permitted intensity. |
| Decision or service | Identity survives while controls and consequential information remain plain and strong. |
| Sequence or journey | Time and position belong to the treatment rather than an unrelated stepper. |
| Product, documentation or index | Dense data and lists retain their information structure and the identity. |
| Opened and generated views | Search, settings, notes, results, empty and error states belong to the page that opens them. Enter and return through the real route. |
| Handheld presentation | The opening and control placement are composed for the available space and input, with the same identity. |

Suppress accent through the roles that actually supply it, including mode-specific aliases, instead of editing scattered rules. Inspect the rendered consumers to confirm suppression; removing the review override must restore the production treatment. Use neutral fixture text and hide identifying labels for the comparison, recording these temporary changes. A grayscale capture can expose hue dependence, but it is a separate diagnostic transformation, not the original page or a substitute for testing accent roles.

Shuffle unlabelled stills, including neutral filenames, for a reviewer who knows the intended identities and did not derive these treatments. Ask them to name each identity and explain which qualities work together to make it recognizable. Record each failing pair, page type, state and reason. A pair that can be distinguished only by accent, a barely nudged value or motion absent from the still has failed this comparison. Restore accent and confirm that it reinforces the distinction. The explanation is a contextual design judgment, not an axis-count certificate.

Ask four questions of a page at rest: can it be recognized without its logo; is any required machine or state presence perceptible without interaction; does its character survive removing hue; and is any visible element merely a prop? Use the quietest page too. Where one answer fails, return to the organizing idea and repair the dimensions it implies, often geometry, raster, type or crop. A louder accent cannot repair a distinction that depends only on color. Recheck the failing pair and the consumers affected by the repair, and put the result in the existing review.

## Keep departures, related destinations and ordinary navigation distinguishable

The current source owns the facts and relation its link states. The target supplies its own identity and destination. Shared infrastructure or a matching style cannot create ownership or membership. Mark a transition where it helps the reader understand what will happen. A small reference can use a light signature, a card can make the relationship more visible, and a true embed can carry more of the target's treatment in its own area. None needs to repaint the host page.

Where the approved design borrows a target signature, scale it to the reference's actual role:

| Reference | Treatment to consider |
| --- | --- |
| Inline mention | A small mark or underline shape; retain the source's reading type and surrounding ink. |
| Preview or citation | Target geometry and metadata treatment within that surface. |
| Card | Target ground, edge and type inflection bounded by the card. |
| Embedded artifact | Scoped target material, texture and meaningful state within the embed. |

Do not promote an inline mention to a card merely to display identity. If a trailing mark is used, keep only the final word and mark together, for example in a `white-space: nowrap` span; reflow at the narrowest supported width and check the result. Keep the rest of a long link free to wrap. Opening and closing a preview use the shared control response, even when the target has a different expressive motion curve.

Keep ordinary activation, modifier clicks, Back and the same essential information with or without a preview. On touch, a separate visible affordance can open a preview. The original destination remains directly reachable. Compare with accent subdued and in forced colors, and check that the cue still says whether the link stays within the current experience or leads elsewhere.

Use native links with `href` for navigation. Inspect the accessible name and description when a mark communicates a necessary destination or departure; add useful text where the existing purpose and context do not convey it. A visual glyph and `rel="external"` do not by themselves supply an accessible explanation. That `rel` value annotates an off-site destination; it does not choose a new tab or establish an object relation. Forced colors removes shadows and non-URL background images, so inspect the remaining mark, underline or form instead of relying on tint or glow. Follow the project's external-preview policy and keep the original link usable when a target signature is unresolved.

## Worked example: distinguish two research indexes

One index uses a shallow matte reading surface, a compact text plane and close, aligned pairs for measurements. The other uses a deeper plate with a figure at each turning point. Their links and search still behave alike. Turn off the accent and compare the same list, article and reader action. If the two differ only in tint, revise the relationship between content, figures and depth before adding a new color.

## Sources

- [WCAG Use of Color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html) and [Link Purpose in Context](https://www.w3.org/WAI/WCAG22/Understanding/link-purpose-in-context.html): distinguish an informative visual cue from the text and context that make a link understandable.
- [WAI-ARIA APG link pattern](https://www.w3.org/WAI/ARIA/apg/patterns/link/) and [HTML external link type](https://html.spec.whatwg.org/multipage/links.html#link-type-external): native navigation behavior and the limited meaning of the destination annotation.
- [MDN forced colors](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/forced-colors) and [white-space](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/white-space): inspect which visual treatments are suppressed and keep a trailing mark with its word without preventing the whole link from wrapping.
