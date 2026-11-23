+++
description = "Use it to find the element that causes overflow by comparing scroll and client extents and inspecting real text at scale."
uses = [{ scope = "local", resource_id = "responsive" }]

[guide]
exposure = "internal"
+++

# Find the element that causes overflow

## Diagnose overflow at its cause

Compare root scroll and client extents under the actual scrolling model, then find escaping boxes, clipped text and fused labels. Do not hide the defect with root clipping or a tolerance chosen to make it pass. A zero-overflow result alone misses content already clipped inside a fixed box. Inspect real text at scale as well as geometry, including long identifiers, titles, translated labels and the project's difficult data.

Choose a container query when a reusable component occupies slots of different width in the same viewport. Choose viewport queries for page-level arrangement. Resolve the actual available inline size rather than shrinking type to force an assumed track minimum. Check intrinsic minimum sizing on flex/grid children, wrapping rules and fixed dimensions before adding an override. A long code token and ordinary prose need different treatment.

For a size query, establish `container-type: inline-size` on the containing slot and use `@container` to adapt its descendants. The component cannot use itself as the size container for its own rule. Render it in a narrow rail and a wide main column at the same viewport width to check that each responds to its actual space.

Two common causes need different repairs:

- An auto-fit grid with a fixed minimum track can overflow when the container is narrower than that minimum. Where the design calls for tracks that can shrink to the available width, use `repeat(auto-fit, minmax(min(100%, var(--track-min)), 1fr))` with the project's actual track token. Inspect the content inside the resulting track too.
- A flex or grid child's automatic minimum can preserve a long string's intrinsic width. Set `min-inline-size: 0` on the responsible item when it needs to shrink, then deliberately wrap the content or give eligible content local scrolling. Removing the minimum alone does not make an unbroken string readable.

For a table, stack records only if their row/column relationships remain usable. Otherwise use an identifiable local scrolling region, keeping headers and access to offscreen content. Meaningful two-dimensional diagrams and code may need similar treatment. The rest of the page must still reflow. Apply the current [Reflow criterion](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html) and its content-specific exceptions: vertical reading at 320 CSS pixels is equivalent to 400% zoom from a 1280-pixel viewport. That equivalence is not a universal equivalence for every starting width or writing direction.

Show that more content exists offscreen and verify keyboard access to the local scroller; provide a suitable label and focusability where its contents offer no keyboard entry. Keep row and column headers identifiable while scrolling. Ordinary headings, prose and forms do not become two-dimensional content merely because a fixed layout is inconvenient. A review board with a wide matrix follows the same rule: the matrix may scroll locally while the surrounding explanation reflows.

Start the browser investigation with the root widths, then inspect boxes extending past the viewport.

```js
const root = document.documentElement;
console.log(root.scrollWidth, root.clientWidth);
[...document.querySelectorAll("body *")].filter((element) => {
  const rect = element.getBoundingClientRect();
  return rect.left < 0 || rect.right > root.clientWidth;
});
```

Use the list as a lead. An intentional local scroller, transform or clipped decoration can put a box beyond the viewport without causing the root to scroll. Inspect the responsible box in context.

## Worked example

A URL makes the whole page scroll sideways. Identify the first overflowing container and its minimum size. Keep the address copyable, allow a useful break or make only the relevant preformatted block scroll. Compare the complete page after text enlargement before concluding the root width is fixed.

## Sources for the diagnosis

- [MDN: container queries](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Containment/Container_queries): establish a size container and query it from descendants.
- [MDN: minimum width](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/min-width) and [grid minmax](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/minmax): distinguish intrinsic child sizing from a track minimum.
- [MDN: scroll width](https://developer.mozilla.org/en-US/docs/Web/API/Element/scrollWidth) and [element bounds](https://developer.mozilla.org/en-US/docs/Web/API/Element/getBoundingClientRect): measure the root and trace candidate escaping boxes without treating every offscreen box as a fault.
