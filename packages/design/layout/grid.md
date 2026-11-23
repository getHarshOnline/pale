+++
description = "Use it to compose the available space with related edges, tracks and a reading measure, and to decide deliberately where to depart from the grid."
uses = [{ scope = "local", resource_id = "layout" }]

[guide]
exposure = "internal"
+++

# Use a grid and decide where to depart from it

## Compose the available space

Choose related edges, tracks, subdivisions and reading measure deliberately. Look for unexplained near-alignments rather than imposing a maximum edge count. Keep long-form text at its intended measure; a wide container can still support figures or useful apparatus beside it. Do not lengthen every line to consume width, invent filler for empty tracks or demand that every margin be occupied. Deliberate quiet can protect reading or stage a meaningful subject.

Use alignment, spacing and hierarchy to establish groups before adding visual enclosures. A border can identify an actionable unit or aid comparison; it is not intrinsically wrong. Temporarily removing it can reveal whether an entire layout relies on boxes to compensate for weak structure. If borders and bands dominate a blurred view, compare that weight with what the content deserves.

Try shared edges before column rules, spacing steps before repeated bands, type hierarchy before boxes around sections, and a material change before a redundant panel outline. One rule can help compare values without enclosing every row. A numbered sequence is appropriate when the content is sequential; it should not manufacture a progression among unrelated items. Keep containers that identify a real record, form, figure or code unit the reader acts on or carries away.

An expressive departure from the normal grid needs a relationship to the surrounding composition and a reason in the brief. An anchored edge, shared baseline or deliberate overlap can explain the departure. Preserve readability, operability and useful recomposition. Conversely, boxing a focal subject at the same scale as every record can make the approved expression ineffective. The project's direction determines permitted intensity; there is no universal one-peak rule, prohibition on centered composition or demand for asymmetry.

Where a page mode sets an expressive allowance, count the departure against it. Keep an identifiable anchor, state what the departure expresses, and protect the reading field, values and controls from overlap. A pinned sequence can be one composed region while its shots change; inspect the actual region rather than counting each frame as a new page decoration. At a narrow width, choose its place, scale and crop again so the effect does not become clipping. A film follows its shot and framing contract rather than importing a page's numerical allowance.

Compose the opening from what the person needs on arrival: the subject, enough orientation and the first useful action or content. Navigation must remain findable without accidentally becoming the main subject. Relate a focal image or object to the claim it introduces; attaching an interchangeable illustration beside text is not evidence that it serves that claim. Compare alternatives with a sketch or existing design surface when the decision needs it. Do not require new drawings for a settled local alignment repair.

At narrow widths, reconsider order, crop, emphasis and apparatus placement from the task. Stacking desktop columns in source order can work, but inspect the actual first view rather than assuming it does. CSS placement changes visual arrangement without repairing logical reading or focus order. If the order no longer makes sense, correct the semantic structure at its owner and recheck each supported presentation. A film's frame uses its own ratio, staging and temporal context; it does not inherit a website's opening checklist.

## Check whether the grid supports the subject

| Relationship you need | Construction to try | Failure to look for |
| --- | --- | --- |
| Related items form a group | Align their edges and use a larger gap after the group. | Lines and bands dominate a blurred capture. |
| Prose keeps its measure on wide screens | Use extra columns for a figure, note or useful apparatus. | Every line grows with the viewport. |
| A figure crosses a margin for a reason | Hold one edge to a known line and give the crossing a clear relation to the text it introduces. | The effect clips the label or hit target, or the figure feels like an unrelated card. |
| The opening works in hand | Sketch the first read, subject and route in their own arrangement. | The reader meets a shrunk object before learning what the page is about. |

Temporarily hide the peak object. If the next content group reads identically and nothing connects to the object, reconsider its relation to the opening. Check departures from the grid against the accepted direction, the real crop and the narrow reading order. Keep surrounding prose, controls and data usable when the object moves.

## Sketch the opening before adding the surface

Identify the claim, the object that carries it and the first action. Draw their order on the narrow layout first. On a wider grid, let the object span beside the claim and action if the reader still meets the same relationships and focus order. Shared tracks and inherited alignment can do the grouping work of borders. A field may break an edge when the break gives the main object a larger reading or expressive space. Anchor it to enough of the grid that nearby captions and controls still have a place.

Compare two readings of asymmetry. A useful departure gives one side a reason to be heavier and leaves somewhere for the eye to go next. An arbitrary offset makes a weak group look unfinished. Temporarily remove the border, extra card or colored divider. If hierarchy disappears, repair the groups, leading edges and gaps before adding decoration back.

## Carry the sketch into CSS

Named `grid-template-areas` make a narrow and wide arrangement easy to compare in both a sketch and the stylesheet. Keep names on the elements and change the area arrangement when the content needs it. Use the project’s real breakpoints; capture widths are test cases, not automatically layout thresholds. Where nested content should share its parent’s tracks, consider `grid-template-columns: subgrid` and verify the sibling edges in the grid inspector. Preserve a usable arrangement on the supported fallback path.

Keep sibling spacing on the parent with `gap` or a scoped sibling rule where that fits the component. This avoids an unintended final margin or doubled padding without imposing a ban on every child margin. Align suitable label/value rows with `align-items: baseline`. Constrain the prose’s measure rather than the whole container when figures or annotations can use the adjacent width. Verify that these constructions preserve logical reading and focus order.

## Worked example

Set the heading, body and caption to shared edges on a twelve-column layout. Let a figure cross the middle tracks if it explains the main claim and its caption keeps the reading edge. Compare the opening without its boxes. On a phone, move the figure after the claim in the reading and focus order, then inspect whether the subject and way forward still appear soon enough. A centred opening can be right when the subject needs it.

## Sources for the decision

- [Carbon 2x Grid](https://carbondesignsystem.com/elements/2x-grid/overview/) and [Fluent layout](https://fluent2.microsoft.design/layout): examples of columns, margins, spacing roles and adaptive regions.
- [Practical Typography: grids](https://practicaltypography.com/grids.html) and [NN/g: common region](https://www.nngroup.com/articles/common-region/): judge alignment and useful containment by the readable result.
- [MDN: grid areas](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_grid_layout/Grid_template_areas), [subgrid](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_grid_layout/Subgrid) and [grid accessibility](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_grid_layout/Grid_layout_and_accessibility): named placement, shared tracks and the distinction between visual and logical order.
- [MDN: item alignment](https://developer.mozilla.org/en-US/docs/Web/CSS/align-items): baseline alignment for appropriate label/value rows.
- [Every Layout: Stack](https://every-layout.dev/layouts/stack/) and [Switcher](https://every-layout.dev/layouts/switcher/): sibling relationships and container-driven row/column composition.
