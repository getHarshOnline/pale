+++
description = "Use it when a dialog, popover or preview opens over the page, to check the modal shell, focus, reading position and drafts when it closes."
uses = [{ scope = "local", resource_id = "access/web" }]

[guide]
exposure = "internal"
+++

# Keep dialogs and previews operable

Use the browser's modal, input and disclosure behavior where it fits. Check what the control makes available and what happens to focus, reading position and drafts when it closes.

Keep one modal shell at a time. A tool-to-tool handoff changes the shell's content while preserving one background veil, one focus container and the original return target.

## Exercise the real opening and return

Name a modal from its visible title. Open it on the action that needs it. Put focus on the first meaningful control, on the start of long content when reading comes first, or on the least destructive action when a confirmation can cause an irreversible change. Check the accessibility tree, then try Tab and Shift+Tab past every control. Try to click and focus the background, scroll inside the modal and verify the page behind it stays put. Test Escape and the visible close button. If outside activation dismisses it, check that activating the backdrop and clicking panel padding have the intended distinct effects. Preserve unsaved input or offer a decision before discarding it. Close a shared shell after handing off between tools and verify focus returns to the original invoker.

For a native `<dialog>`, use `showModal()` when the background must be inactive and choose initial focus deliberately, using `autofocus` where appropriate. A close control inside a `<form method="dialog">` can close the dialog through browser behavior; use it for dismissal, keeping real submission on its intended route. Test the browser’s focus return before adding custom restoration code. If the design offers outside-click dismissal, `closedby="any"` supplies it where supported; otherwise distinguish coordinates outside the dialog's bounding box from clicks on its own padding. A click whose target is the dialog element alone does not make that distinction. Check support in the target browsers and keep Escape and the visible close control working.

Use a combobox only when the input offers choices in a popup. Choose the matching popup pattern. With a listbox popup, keep DOM focus in the input, connect the popup with `aria-controls`, publish its open state with `aria-expanded`, and use `aria-activedescendant` for the active option. Update that relationship as options disappear. Arrow and Enter handling must leave input method composition alone. Escape dismisses the popup before a second Escape reaches the surrounding shell. A destination list made of ordinary links can use a search field and normal links. Keep browser modifier clicks and the context menu available on genuine links. A listbox option is unsuitable for holding an independently operable link.

Keep a region with `role="status"` in the document before updating a result count, empty result, loading state or failure. It announces updates politely without moving focus. Add explicit `aria-atomic="true"` when the whole message and its context must be announced, and test that the supported screen reader announces the settled result once. When a preview appears on hover or focus, let Escape dismiss it, let the pointer enter it without collapse, and leave it visible while the trigger or preview is in use. Keep the underlying link usable. Give touch a visible preview action if a preview is offered. Keep focus access when a keyboard is connected to a touch device. Long press alone leaves no discoverable path. Check an actual wide and narrow journey through open, type, move, follow, Back, dismiss and reopen.

Keep preview text within the viewport and readable over the composited background without shifting the paragraph that opened it. Make its content and any controls available to assistive technology. Apply a hover-intent delay only to pointers that can hover; keyboard access remains immediate. A neighbouring preview can reuse the already-open interaction instead of forcing the reader to wait again. The project's motion settings supply the timing.

## Worked example

Tab into a named dialog and through its controls in both directions. Escape and the visible Close control each return focus. In the search field, composing text does not activate a result. For a separate preview, move the pointer onto it, dismiss it without leaving the reader’s place, and try the keyboard and touch routes. The original link remains reachable throughout.

Repeat dismissal and reopening rapidly, and try a handoff within the shared shell. In the review, say which opening, dismissal, navigation and return paths worked, and give the failing path and observed focus or scroll change for each defect.

## Implementation sources

- [HTML dialog](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-dialog-element): native modal opening, initial focus and closing behaviour.
- [MDN dialog](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/dialog): `closedby` values and browser support.
- [WAI listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/): option navigation and the limits of interactive content inside options.
- [WAI disclosure pattern](https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/): content that needs an operable disclosure rather than a tooltip.
- [WAI dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/): initial focus, containment, naming and return after closing.
- [WAI combobox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/): input and popup roles, states, active options and Escape behavior.
- [Content on hover or focus](https://www.w3.org/WAI/WCAG22/Understanding/content-on-hover-or-focus.html): dismissal, persistence and entering the preview with the pointer.
- [Status messages](https://www.w3.org/WAI/WCAG22/Techniques/aria/ARIA22): create the region before updating its content, and decide whether the full message needs announcing.
