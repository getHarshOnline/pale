+++
description = "Implement complete browser access through forms, widget semantics, focus, dialogs, navigation and announcements."
uses = [{ scope = "local", resource_id = "access" }]
see_also = [{ scope = "local", resource_id = "access/keyboard-focus" }, { scope = "local", resource_id = "access/dialogs-previews" }, { scope = "local", resource_id = "access/contrast-targets" }, { scope = "local", resource_id = "access/motion-accessibility" }]
official_sources = [{ label = "W3C Web Content Accessibility Guidelines 2", url = "https://www.w3.org/WAI/standards-guidelines/wcag/", jurisdiction = "International web standard; verify applicable local law separately", checked = "2026-08-04" }]

[guide]
+++

# Make the browser task accessible

Use this when designing, changing or checking a browser task. Start with built-in elements whose behavior fits. A custom control needs a name, role, states, input, focus, recovery and announcements.

Preserve the person’s input and a useful path after a form error. Make each progress or state change understandable without color, sound, motion or a fine pointer. Check the complete task with the input and assistive technology it needs.

| If you are doing this | Read this |
| --- | --- |
| Check names, order, clipping, shortcuts or focus return | [Keep keyboard access and focus through a task](keyboard-focus.md) |
| Open a dialog, combobox or hover/focus preview | [Keep dialogs and previews operable](dialogs-previews.md) |
| Measure text, focus, state cues or hit regions | [Check rendered contrast and pointer targets](contrast-targets.md) |
| Check reduced motion, moving content or playback controls | [Keep the task available when motion changes](motion-accessibility.md) |

## Before handing off

- [ ] Can the task be completed by keyboard and the other supported inputs tested?
- [ ] Do names, focus, announcements and recovery match what the controls do?
- [ ] Were contrast and pointer targets measured on the rendered surfaces?
- [ ] Does the reply say which paths worked, which failed and which remain untested?
