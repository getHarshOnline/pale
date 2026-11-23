+++
description = "Adapt an interface to changes in space, input, zoom and available capabilities while preserving its task, content and design."
uses = [{ scope = "local", resource_id = "access/web" }]
see_also = [{ scope = "local", resource_id = "responsive/order" }, { scope = "local", resource_id = "responsive/overflow" }, { scope = "local", resource_id = "responsive/viewport" }, { scope = "local", resource_id = "responsive/capabilities" }]

[guide]
+++

# Recompose an interface without losing its task

Use this when available space, input or capability changes an interface’s arrangement. Decide what the reader must still see and do. Treat width, orientation, zoom, hover and the visible viewport as separate inputs.

| If you are doing this | Read this |
| --- | --- |
| Reorder or recompose real content | [Preserve meaning as the layout changes](responsive/order.md) |
| Repair a long string, grid track or scrolling child | [Find the element that causes overflow](responsive/overflow.md) |
| Handle touch, keyboards, zoom or changing viewport | [Design for the visible viewport and its input](responsive/viewport.md) |
| Test script, preference, first paint or persistence | [Keep the task when a capability is absent](responsive/capabilities.md) |

## Before handing off

- [ ] Are reading order, focus order and real input useful after rearrangement?
- [ ] Is overflow fixed at its source without concealing required content?
- [ ] Does the original task work at the relevant sizes, settings and capability fallbacks?
