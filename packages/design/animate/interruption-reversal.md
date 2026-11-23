+++
description = "Use it to decide what advances each moving part and how it handles a new input, an offscreen part or a pause."
uses = [{ scope = "local", resource_id = "animate" }]
see_also = [{ scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "animate/time-models" }, { scope = "local", resource_id = "animate/retarget-controls" }, { scope = "local", resource_id = "animate/pause-and-resume" }]

[guide]
exposure = "internal"
+++

# Keep motion responsive when its input changes

Choose what advances the motion, then decide how it handles new input, invisibility and pause. A drawer follows the latest request, a scrubbed sequence follows the reader’s position, and a film keeps its playhead.

| If you are doing this | Read this |
| --- | --- |
| Decide what advances each moving part | [Choose what advances the motion](time-models.md) |
| Handle a new input before the old transition finishes | [Reverse or retarget a control in flight](retarget-controls.md) |
| Suspend work when it goes offscreen or the reader pauses it | [Pause work and resume from the same state](pause-and-resume.md) |
