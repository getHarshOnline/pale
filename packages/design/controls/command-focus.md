+++
description = "When opening, navigating or closing search, a command dialog or a result popup."
uses = [{ scope = "local", resource_id = "controls" }]

[guide]
exposure = "internal"
+++

# Keep focus and input working in search

Use this when opening, navigating or closing search, a command dialog or a result popup.

For an editable combobox with a listbox popup, keep its focus and active-option model aligned with the [WAI-ARIA Authoring Practices pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/). Keyboard and pointer must identify one intended result; Enter must not open another result. Do not consume input-method composition as a command. Other popup models have different focus rules. Preserve real links where results offer destinations and do not swallow modified clicks. A result should reveal what it is and where activation leads before the person commits.

Use the project’s chosen dialog or popup pattern. For a modal dialog, keep its focus inside until it closes and return focus to the opener or an intentional successor. An anchored nonmodal surface has different rules. When a result links to a page, keep the actual link reachable. Do not allow hover to change which item Enter activates without also updating the focus model and visible active item.

## Worked example: search while composing text

Open the search dialog from the keyboard. Compose a word through an input method and press Enter to confirm the composition. Search stays open and keeps the entered text. After composition ends, Arrow Down moves to a visible result, and Enter follows that result. A modified pointer click on a link keeps the browser’s new-tab behavior. Escape closes the dialog and returns focus to the opening control.
