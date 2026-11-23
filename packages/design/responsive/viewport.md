+++
description = "Use it to design for the viewport people actually have, including changing browser chrome, safe-area insets, zoom and text resizing."
uses = [{ scope = "local", resource_id = "responsive" }]
see_also = [{ scope = "local", resource_id = "responsive/overflow" }]

[guide]
exposure = "internal"
+++

# Design for the visible viewport and its input

## Meet the viewport people actually have

Let a full-screen region grow when its content exceeds the viewport. Select small or dynamic viewport sizing according to whether it should remain stable or track changing browser chrome; verify the resulting browser behavior. Account for safe-area insets on every affected edge and after rotation. Do not duplicate the owning frame's viewport declaration in child components.

Keep user enlargement available. Test page zoom and text resizing as separate failure probes when the supported platform permits it: layout reflow can pass while fixed text boxes clip. Under the applicable [Resize Text criterion](https://www.w3.org/WAI/WCAG22/Understanding/resize-text.html), text must support enlargement to 200% without losing content or functionality, subject to the criterion's exceptions. Viewport-only type scaling can defeat the intended enlargement. Check spacing overrides and actual fallback fonts as well; a nominal size is not proof of readable rendered text.

For forms, test the last field of the longest real flow with the software keyboard open. Keep the field, label, error and useful action visible, allow scrolling, and preserve drafts through rotation and dismissal. Input mode and autocomplete should match the data. Browser-specific keyboard/viewport behavior and platform auto-zoom need observed evidence; a token name does not prove them.

Distinguish the visual viewport from the layout viewport. A software keyboard can shrink the visible region without resizing the layout. The viewport meta's `interactive-widget` setting offers resize behaviors, but support and behavior must be checked on the target; a dynamic viewport unit alone does not prove that a field clears the keyboard. Keep inline errors beside their fields as well as any summary, and use a legible input role rather than suppressing the user's zoom to conceal focus-time enlargement.

Check landscape with its real sticky header and bottom controls. Run the [Find the element that causes overflow](overflow.md) at the widths and zoom the task needs. Compare focus order with visual order and exercise navigation and search by touch and keyboard. A useful intermediate width can expose a defect the smallest and largest views both miss.

For a full-height introduction, compare `min-block-size: 100svh` with `100dvh`: the small viewport gives a stable baseline while the dynamic unit follows browser chrome. Use minimum sizing when content may need more room. Keep safe-area padding at least as large as both the design gap and the device inset, for example `padding-block-end: max(var(--page-gap), env(safe-area-inset-bottom))`. Check the real target after rotation and keyboard opening instead of treating these declarations as proof.

When the owning frame uses `viewport-fit=cover`, backgrounds may reach the device edge while text and controls respect its insets. Check all affected sides and rotate both ways; padding only the top misses a landscape notch. Keep sticky chrome short enough to leave reading space on a short, wide viewport. Do not lock orientation unless the task has an essential orientation requirement.

A hybrid device can have a fine primary pointer and an additional coarse pointer. Where touch needs larger targets or more spacing, `(any-pointer: coarse)` can select that treatment; `(pointer: coarse)` describes only the primary pointer. Gate hover-only visual effects with the appropriate hover and pointer capabilities, and keep the underlying action available by touch and keyboard. Neither width nor a pointer query establishes a person's device or preferred input.

## Worked example

Open a form on a phone and focus the last field. When the software keyboard opens, verify that the control and its error are reachable and that a fixed action does not cover the focus target. Repeat with a narrow desktop and its pointer. The narrow desktop still has hover and can use a different enhancement.

## Sources for the decision

- [CSS viewport units](https://www.w3.org/TR/css-values-4/#viewport-relative-lengths) and [environment variables](https://www.w3.org/TR/css-env-1/): size the region and account for device insets as separate concerns.
- [MDN: viewport meta](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/meta/name/viewport): viewport fitting, keyboard resize choices and enlargement controls.
- [HTML autofill](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#autofill) and [MDN: inputmode](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Global_attributes/inputmode): declare field purpose and suitable keyboard hints.
- [MDN: pointer](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/pointer) and [any-pointer](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/any-pointer): primary versus additional input capabilities.
- [W3C: orientation](https://www.w3.org/WAI/WCAG22/Understanding/orientation.html): supported orientation and the essential-use exception.
