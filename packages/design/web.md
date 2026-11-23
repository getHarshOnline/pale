+++
description = "Build or repair web templates, styles, modules and resource delivery with context-appropriate encoding, complete dependencies and measured performance."
uses = [{ scope = "local", resource_id = "design" }]
see_also = [{ scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "type" }, { scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "web/change" }, { scope = "local", resource_id = "web/content" }, { scope = "local", resource_id = "web/dependencies" }, { scope = "local", resource_id = "web/performance" }, { scope = "local", resource_id = "web/states" }, { scope = "local", resource_id = "web/navigation" }, { scope = "local", resource_id = "web/floating-layers" }]

[guide]
+++

# Carry a design through the web build

Use this when implementing or diagnosing templates, styles, modules or delivered assets. Read the project’s current components, tokens and tool manuals. Decide which source produces the result and check what the browser actually receives.

This is implementation craft: native semantics, responsive input, delivery and measured runtime cost. Direction comes from the brief and selected references. A technique does not earn its place because it is fashionable, and passing these checks does not establish a considered design. Preserve the project's promised usable base when enhancements fail; an application that requires a capability needs an honest unavailable state. Shared interface values belong to their design roles, while authored geometry and algorithm constants belong to their mechanisms.

| If you are doing this | Read this |
| --- | --- |
| Place a template, component, style or delivery change | [Find the source that should own a web change](web/change.md) |
| Put supplied text, data and URLs into safe, meaningful markup | [Render content in the appropriate output context](web/content.md) |
| Trace modules, styles and resource loading | [Load the dependencies and assets the page uses](web/dependencies.md) |
| Diagnose loading, layout or input delays | [Measure a web performance problem where it occurs](web/performance.md) |
| Implement control states or a dialog transition | [Apply input states and transitions in CSS](web/states.md) |
| Add continuity to browser navigation | [Use a view transition without losing navigation](web/navigation.md) |
| Implement a filtered floating layer | [Render a floating layer with a legible fallback](web/floating-layers.md) |

## Before handing off

- [ ] Is the correction in the source that produces the content, presentation or delivery setting?
- [ ] Do encoding and reading order suit the actual contexts?
- [ ] Do styles, fonts and modules resolve in the delivered page?
- [ ] Does the measured result resolve the reported symptom?
