+++
description = "Build an interactive figure, staged object or procedural region with accurate data, usable input, a complete fallback and controlled rendering."
uses = [{ scope = "local", resource_id = "access/web" }]
see_also = [{ scope = "local", resource_id = "figures/compose-layers" }, { scope = "local", resource_id = "figures/drive-a-field-with-data" }, { scope = "local", resource_id = "figures/map-input" }, { scope = "local", resource_id = "figures/debug-the-drawing-layer" }, { scope = "local", resource_id = "figures/check-a-field" }, { scope = "local", resource_id = "figures/webgl" }, { scope = "local", resource_id = "figures/authored-players" }, { scope = "local", resource_id = "figures/budget-a-scene" }]

[guide]
+++

# Compose and check a live visual region

Use this for an interactive figure, procedural field or staged object on a page. Decide what the region contributes, how its content reaches the reader and how it responds when the reader or device changes. A film can use the same design reasoning and has its own render and playback choices.

| If you are doing this | Read this |
| --- | --- |
| Choose a substrate, plate, bleed or static image | [Choose the relation between a live region and its page](figures/compose-layers.md) |
| Represent a mechanism, real data or an authored object | [Keep data, models and decoration legible](figures/drive-a-field-with-data.md) |
| Choose a still, map pointer or focus, or separate two clocks | [Keep a meaningful still and map input to the live region](figures/map-input.md) |
| Budget memory, profile a field, suspend work or recover from context loss | [Spend rendering work where the reader can see it](figures/debug-the-drawing-layer.md) |
| Check input, fallback, performance or truth in the running result | [Exercise a figure in the page that uses it](figures/check-a-field.md) |
| Size, draw or recover a WebGL region | [Run and recover a WebGL drawing region](figures/webgl.md) |
| Play authored animation without losing the still | [Deliver an authored animation with a controlled player](figures/authored-players.md) |
| Choose between a still, SVG, canvas and a scene graph | [Choose and budget a drawing layer](figures/budget-a-scene.md) |

## Before handing off

- [ ] Does the figure, field or object contribute what the brief intended?
- [ ] Are facts, labels and destinations available in both still and live views?
- [ ] Do input, the selected clock, fallback and return behavior work in the page?
- [ ] Does the busiest view meet its budget and recover from drawing failure?
