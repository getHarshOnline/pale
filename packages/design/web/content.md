+++
description = "Use it to render supplied content safely in the right output context, with the template dialect, escaping and missing values confirmed."
uses = [{ scope = "local", resource_id = "web" }]
see_also = [{ scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "access/dialogs-previews" }, { scope = "local", resource_id = "web/states" }]

[guide]
exposure = "internal"
+++

# Render content in the appropriate output context

## Render the supplied model safely

Confirm the template dialect, enabled features, context per render entry and actual escaping configuration. A matching language name does not import another generator's globals or a later major version's syntax. Distinguish guaranteed fields from optional values; missing, null, empty and zero need their intended treatment. A default filter may not handle all four equally.

Present supplied facts and order. Keep authoritative relationships, dates, totals and routes at their model owner rather than reconstructing them by parsing HTML, guessing a URL or iterating incidental object order. Permitted formatting follows the source contract. Pass reusable fragments their actual inputs and declare their imports instead of depending on ambient caller context. A block contract should expose useful composition seams without requiring a whole-frame fork or an override for every attribute.

Autoescaping is not a context-aware sanitizer. Use the encoder appropriate to HTML text, quoted attributes, URLs, script data or CSS; keep names and program structure trusted. Attribute escaping does not validate a URL scheme. A raw JSON serializer marked safe can still cross the enclosing script boundary. Use the context-specific encoder and supported safe sink. Bypass escaping only for the trusted rendered type the contract admits. Avoid encoding twice or stripping tags to recover a plain-text field the model already supplies. Whitespace in code, text controls and inline runs can be meaningful; inspect it as content. [OWASP output-encoding guidance](https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html)

Use relevant fixtures for optional fields, list boundaries, differing render payloads and branches a change can reach. A filter or import error may appear only when that branch renders. A standalone renderer can isolate a language problem, while the application still owns its limits, encoders and output checks. Confirm the correction through that application. For reproducibility, bind external inputs and compare resulting bytes; do not silently introduce clock, environment, network or random values into an otherwise captured render.

## Worked example

An article title appears in a heading and a JSON object. Use the template’s text escaping for the heading and its context-safe JSON output for the data. If JSON sits inside an HTML script element, also test a title containing `</script>` so serialization cannot terminate that element. Check quotes, angle brackets and non-Latin characters in both outputs. Keep the title and destination real when a browser enhancement fails to load.

## Choose markup that already carries the behavior

Use a real link for a destination and a button for an action. A disclosure can use `details` and `summary`; a nonmodal popover can use `popover` and a `popovertarget` button. A popover does not make the background inert or supply a modal focus contract. For modal opening, initial focus and native `<form method="dialog">` dismissal, follow [Keep dialogs and previews operable](../access/dialogs-previews.md). Where the delivery contract promises a usable page without script, keep a real destination or inline route to the same content when scripted opening is unavailable.

Style existing state attributes such as `aria-pressed`, `aria-current`, `aria-expanded` and `[open]` instead of maintaining a second visual flag. Keep those attributes accurate; CSS cannot supply the underlying behavior. Use `:user-invalid` when invalid styling should wait for interaction rather than flagging untouched required fields on load. It does not replace validation, an explained error or the submitted form's recovery path. See [Apply input states and transitions in CSS](states.md).

| Inspect in the emitted page | Correction when missing |
| --- | --- |
| Main content, repeated navigation and reading order | Supply the main landmark; distinguish repeated landmarks by accessible names where needed. Use headings for structure, with size chosen separately. |
| Keyboard entry, language and direction | Provide a working skip link; declare the document language and changes of language or direction within it. |
| Links and dates | Make link text understandable in context and preserve real destinations; encode dates with `time datetime` at their supplied precision. |
| Images, figures and media boxes | Supply appropriate alternative text or a complete text equivalent; keep decoration silent and reserve the media's dimensions. |
| Data tables | Use a caption where it identifies the table, headers and correct associations; complex tables may need more than `scope`. Do not use tables to arrange unrelated page content. |
| Forms | Give controls visible labels, suitable `type`, `inputmode` and `autocomplete`, and related groups a `fieldset` and `legend` where appropriate. A placeholder is not a label. |
| Errors and status | Explain errors beside the relevant fields and associate the explanation; expose invalid state after validation and give a useful route to errors after submit. Keep routine status in a live region established before the update, and reserve alerts for urgent information. |

Inspect the accessibility tree and complete the task through [Make the browser task accessible](../access/web.md). A source selector or scanner result cannot establish that the names, order and announcements work when heard. Renderer-specific rejected markup belongs in the renderer's own documentation; use its supported representation instead of bypassing the check.

## Sources

- [WAI page structure](https://www.w3.org/WAI/tutorials/page-structure/): landmarks, headings and navigation within a document.
- [WAI forms](https://www.w3.org/WAI/tutorials/forms/): labels, grouping, validation and notifications.
- [WAI images](https://www.w3.org/WAI/tutorials/images/) and [tables](https://www.w3.org/WAI/tutorials/tables/): alternatives and header associations suited to the actual content.
- [MDN Popover API](https://developer.mozilla.org/en-US/docs/Web/API/Popover_API): declarative triggers, dismissal and the distinction from a modal dialog.
- [MDN `:user-invalid`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Selectors/:user-invalid): when validation styling begins and how it differs from `:invalid`.
