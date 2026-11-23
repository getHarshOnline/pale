+++
description = "When choosing a search entry, shortcut, shared dialog, persistent panel or result page."
uses = [{ scope = "local", resource_id = "controls" }]

[guide]
exposure = "internal"
+++

# Choose where search and commands belong

Use this when choosing a search entry, shortcut, shared dialog, persistent panel or addressed result page.

A reader should reach navigation and essential destinations without learning a shortcut or using a model. A shortcut accelerates a visible path. Keep ordinary links as links with addresses, history, new tabs and modifier activation. Use buttons for actions. A local index can support deterministic search while an optional assistant is unavailable.

A command surface can suit fast transitions between tasks. Keep one shared shell when search and commands already use the same focus and dismissal behavior. If a dialog needs to hand off to a second instrument, close or replace the first and move focus intentionally. Put frequent, visible actions on the page when hiding them would make the reader work harder. A permanent site tool belongs where it supports the reading flow, rather than covering it by default.

## Worked example: open search from an article

The header contains a Search button and a keyboard shortcut. Both open the same search surface and put focus in its field. A result can be opened normally or in a new tab. Escape returns focus to Search and leaves the article at its previous scroll position. If the same person opens search on touch, the button still reaches it and the page keeps its direct links.
