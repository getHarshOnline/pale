+++
description = "Use it to check links and buttons, the skip link, tab order and where focus goes after opening, validation, removal and close."
uses = [{ scope = "local", resource_id = "access/web" }]

[guide]
exposure = "internal"
+++

# Keep keyboard access and focus through a task

Use links for navigation and buttons for actions. Keep normal link destinations, modifier activation, new tabs and context menus working when you add previews. Observe where focus goes after opening, validation, removal, handoff and close. Use real navigation and Back when the task needs the reader to return to a previous position.

## Watch each stop at the surface it reaches

Tab from the top of the page. Check that a skip link appears at focus, reaches the main content, and continues in reading order. Compare focus with visual and DOM order at narrow and wide sizes. Positive `tabindex` can make the three orders disagree. Receiving focus must not navigate, submit a form or open a modal. Keep keyboard actions available without a required speed or duration for individual keystrokes; the keyboard criterion’s exception concerns an underlying function that depends on the movement path, such as freehand drawing, rather than merely its endpoints. In a composite, arrows can move among choices and Tab should leave the widget. A modal contains focus while open and returns it to its invoker, or the nearest logical successor if the invoker disappeared. Check the scroll position along with the focus target.

Keep focusable controls and their ancestors out of `aria-hidden="true"`; it hides semantics without removing keyboard focus. When a whole inactive region must be unavailable, use the supported modal behavior or `inert` and test its focus path.

Read the computed name as well as the visible label. Put the visible wording first in the accessible name so a speech-input user can call it. Inspect names on controls with icons, hidden labels and repeated visible text. Do the same action with a pointer, keyboard and, for a drag, a single tap or click path when the drag is nonessential. A keyboard shortcut is an extra route. Keep it off editable fields and input method composition unless it belongs there. Single-character shortcuts need the applicable off, remap or focus-only behavior.

When a control needs supplementary label text that should remain available to assistive technology, use a visually hidden treatment rather than `display: none`, `visibility: hidden` or the HTML `hidden` attribute. One pattern puts the text in an absolutely positioned 1 × 1 px box with `overflow: hidden`, `white-space: nowrap` and `clip-path: inset(100%)`. Keep the control itself visible, avoid repeating context in every hidden label, and confirm the computed name in the accessibility tree. A hidden skip link must become visible when focused.

Check focus on a picture, light ground, dark surface and beneath sticky headers or banners. A two-tone drawing can survive changing backgrounds. An outline or border can survive forced colors where a box shadow disappears. Test under actual scrolling, and listen to the names, roles and states using the supported browser and screen reader. A scanner cannot establish that the order makes sense when heard.

For a modal, press Tab more times than there are focusable controls, then repeat with Shift+Tab. Focus should stay inside until dismissal. After closing, check four things: focus returns to the invoker or its logical successor; the page retains its scroll position; selections, disclosure states and drafts survive; and navigating to a result and using Back restores the previous reading position. Include the observed before-and-after focus and scroll values in the review when reporting a restoration defect.

## Recognise common failures

| What you see | Repair |
| --- | --- |
| `outline: none` with no replacement | Restore a visible focus indicator. |
| An accent-coloured ring disappears on an accent surface | Measure against both the control and its surround; use a contrasting band or another indicator that works there. |
| Some controls have focus styling and others do not | Walk the whole task and fix the missing component states. |
| Focus scrolls behind a sticky header or footer | Reserve space with `scroll-padding-block-start` or `scroll-padding-block-end` on the scrolling container, using the actual obstruction size. |
| Positive `tabindex` or responsive reordering breaks the reading sequence | Repair the document order and layout together. |

Try the screen-reader and browser combinations the project supports, such as VoiceOver with Safari or NVDA with its supported browser. Use their heading and landmark navigation as well as Tab. In the review, name the versions, the route tried and any names, roles or states that were missing or misleading.

## Worked example

Open a drawer from the fourth link in a list. Tab forward and backward. Resize the view so a sticky bar can cover the focused item. On close, return focus and the reader’s position to the trigger. Remove the triggering item during a second run and check that focus lands on the next useful item. Match the visible label to the accessible name so a speech-input command can use it.

## Sources

- [Keyboard](https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html) and [No Keyboard Trap](https://www.w3.org/WAI/WCAG22/Understanding/no-keyboard-trap.html): keyboard operation, timing and intentional modal containment.
- [Focus Order](https://www.w3.org/WAI/WCAG22/Understanding/focus-order.html), [Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html) and [Focus Not Obscured](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html): reading order, indicators and sticky obstructions.
- [Label in Name](https://www.w3.org/WAI/WCAG22/Understanding/label-in-name.html): matching the visible wording for speech input.
- [Character Key Shortcuts](https://www.w3.org/WAI/WCAG22/Understanding/character-key-shortcuts.html): when a shortcut needs an off, remap or focus-only option.
- [Dragging Movements](https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html): single-pointer alternatives and the essential-drag exception.
- [On Focus](https://www.w3.org/WAI/WCAG22/Understanding/on-focus.html): context changes that must wait for activation rather than focus.
- [Supplemental link text](https://www.w3.org/WAI/WCAG22/Techniques/css/C7): visually hidden context that remains available to assistive technology without hiding the link.
- [Forced colors](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/forced-colors): properties the browser replaces or removes, and system colors for controls that need custom states.
- [WAI widget patterns](https://www.w3.org/WAI/ARIA/apg/patterns/): the keyboard model for the particular custom control being built.
- [VoiceOver testing](https://webaim.org/articles/voiceover/) and [NVDA testing](https://webaim.org/articles/nvda/): heading, landmark and control navigation, including the difference between reading and interacting.
