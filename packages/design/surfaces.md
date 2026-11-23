+++
description = "Choose or repair surface depth, texture, translucency, scene light and state-bearing light, including their fallbacks and return to rest."
uses = [{ scope = "local", resource_id = "access" }]
see_also = [{ scope = "local", resource_id = "surfaces/choose-reflection-or-emission" }, { scope = "local", resource_id = "surfaces/choose-a-surface" }, { scope = "local", resource_id = "surfaces/check-the-surface" }, { scope = "local", resource_id = "surfaces/without-travel" }]

[guide]
+++

# Choose what material and light communicate

Use this when a surface, edge, shadow or light needs a job. Decide whether it explains depth, presents a scene, helps reading or shows an actual state. Check the material and state in the running interface or finished frame.

| If you are doing this | Read this |
| --- | --- |
| Show an actual held state or choose a material accent | [Separate a reflection from a state light](surfaces/choose-reflection-or-emission.md) |
| Design stock, a plate, a raised control or a floating layer | [Give surfaces a material and depth relationship](surfaces/choose-a-surface.md) |
| Find weak contrast, missing blur or expensive compositing | [Test translucency and texture where content moves](surfaces/check-the-surface.md) |
| Adapt to reduced motion, reduced transparency or forced colors | [Keep light and state when the effect changes](surfaces/without-travel.md) |

## Before handing off

- [ ] Does each surface and light help the reader perceive the intended relationship?
- [ ] Can scene illumination, emphasis and actual state be distinguished?
- [ ] Are text and controls legible on the limiting surface, with acceptable backdrop cost?
- [ ] Is the state clear without its glow or movement?
