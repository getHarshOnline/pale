+++
description = "Use it to inspect a layout on the rendered page with a grid overlay, computed geometry or a blurred capture, and to correct what you find."
uses = [{ scope = "local", resource_id = "layout" }]

[guide]
exposure = "internal"
+++

# Check a layout on the rendered page

## Inspect and correct the rendered composition

Choose observations that answer the defect. A grid overlay can expose track drift; computed geometry can measure a spacing bond; an unadorned or blurred capture can reveal relative masses. These are diagnostic views, not mechanical quality scores. Compare the visual prominence with the intended content order, then inspect the complete readable result at scale.

Optical alignment can differ from bounding-box alignment: a round glyph, pointed icon or leading quotation may need an optical correction. Check the actual font and neighboring content before nudging. Do not claim visual precision from values typed into a review board. When layout or typography changes, recheck the affected widths, real content stress and access conditions. Preserve source values, text-spacing resilience, logical reading order and target room while making the composition stronger.

Correct the owning layout or recurring recipe, then inspect its actual consumers. Keep a margin that serves reading. Remove an accidental override after its replacement works. In the existing review or reply, name remaining design questions or platform limits beside the behavior they affect.

Use the browser’s grid inspector with area names and extended lines to compare actual edges and tracks. Read `getBoundingClientRect()` for visible blocks when several nearly aligned edges are hard to distinguish. Consolidate accidental near-alignments; the number of edges is not a composition limit. Rank the groups first by content importance, then by weight in a blurred capture, and compare the orders. Read the normal, unblurred page before deciding on the repair.

Inspect the first view separately at wide and narrow widths: name what the reader can understand or do and compare it with the page's task. Word count does not judge an opening. For a collection, find an item by scanning one edge; for a comparison, keep values and their qualifications together; for reading, check the measure and the surrounding apparatus. An accidental orphan must not turn an equally important item into the featured one. Label each large gap with its job, and use the [density method's pair checks](density.md#get-density-and-small-relationships-right) for the small bonds that establish belonging. Inspect those bonds again in pressed, focused and error states.

## Worked example

A heading’s bounding box aligns to the grid but its first rounded letter appears inset. Compare the actual outline against the reading edge. Then blur a screenshot to see whether the surrounding frame overshadows the title. Check the finished page at ordinary scale and resize it with long copy. Remove a border temporarily to distinguish a grouping defect from a frame that is simply too heavy.

## Sources for the check

- [Chrome grid inspector](https://developer.chrome.com/docs/devtools/css/grid): show area names, tracks and extended lines against the delivered layout.
- [MDN: element bounds](https://developer.mozilla.org/en-US/docs/Web/API/Element/getBoundingClientRect): measure rendered position and size; geometry does not settle optical alignment.
- [Optical alignment examples](https://rafaltomal.com/optically-perfect/): compare round and pointed forms with their mathematical boxes, then judge the result at native scale.
