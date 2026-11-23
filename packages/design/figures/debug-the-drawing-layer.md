+++
description = "Use it to spend rendering work where the reader can see it, keeping optional heavy initialisation from delaying essential text and controls."
uses = [{ scope = "local", resource_id = "figures" }]

[guide]
exposure = "internal"
+++

# Spend rendering work where the reader can see it

## Bound work and recover the right resources

Keep optional heavy initialization from delaying essential text and controls. Start it near actual use when that serves delivery, retain the composed result until ready, and measure the real load and interaction path. Separate event capture from drawing work: coalesce updates appropriately instead of performing expensive rendering on every raw movement. Use the selected clock's elapsed time rather than display-frame count for cadence.

Measure resolution, pass count, particles, memory and main-thread work at the actual target. More device pixels are not always perceptibly better. Use the lowest sufficient quality within the accepted treatment and define a useful degradation order; an explanatory role and an atmospheric role do not acquire fixed priority from their names. Check edges and text at the lowest delivered quality. A performance optimization that erases the requested visual result does not preserve it.

Separate production rendering from audience playback. A 3D scene can produce a still or film without sending a live scene graph to the page; an interaction can need that geometry at runtime. Compare actual dependencies and preserved behavior rather than classifying a field from its name. For live scenes, count draw calls, textures, shadow and postprocessing passes at the busiest view. Share or instance compatible repeated objects, and bake static relationships only while the selected light and geometry remain valid.

Budget decoded resources rather than compressed file sizes. An RGBA8 frame's base pixel storage is width × height × 4 bytes; formats, mipmaps, render targets, buffering and decoder overhead can change total residency. A scrubbed video's seek cost depends on its encoding and keyframe spacing. Compare the needed working set and both seek directions on the target instead of preloading an entire long sequence by default. Read texture and output color-space contracts before compensating for a washed-out image by changing its palette.

Reuse allocations where appropriate and release owned geometry, textures, programs, targets and observers through the lifecycle. Removing a scene node does not necessarily release its GPU resources. Shared assets need their actual ownership policy; disposing another live consumer's resource is not cleanup. The API and measured target determine the strategy.

Suspend optional drawing when unseen or hidden where appropriate, and render on meaningful change when nothing is evolving. Resume from current semantic state without an unwanted entrance. A selected visible ambient clock may need continuous work; required background computation and offline rendering have different lifecycles. The runtime owns scheduling, cancellation, cleanup and any network or model permission. Do not introduce a remote runtime, model download or credential path merely to decorate a surface.

Test initialization failure and context loss through an authorized native reproduction. Keep the useful fallback, stop invalid work, then rebuild the resources the selected graphics API requires before resuming. Do not assume old buffers or textures survive restoration. Preserve content and avoid repeated retry loops that consume resources without a viable capability. Observe recovery on the actual implementation; a source handler alone proves no resumed output.

Treat performance as part of the visible design. A beautiful still does not compensate for stutter, heat or delayed text in the intended use. Record the chosen degradation steps: for example, remove an unnecessary secondary pass, reduce particles, lower a pass's resolution or cadence, then use the composed still if the live form cannot meet the budget. The order follows this treatment's meaning, and every retained step needs inspection. Use `IntersectionObserver` to detect approach and departure and the Page Visibility API for a hidden document; these are separate conditions. Keep callbacks short and teardown complete.

## Worked example

A 1920 × 1080 RGBA8 target uses 8,294,400 bytes, about 7.9 MiB, before buffers, mipmaps and extra passes. Count two targets and the decoded textures before diagnosing from compressed download size. Observe the busiest view, then lower pixel density or cut an unneeded pass and compare edges. Use the drawing API’s test path to lose context. Stop the loop, keep the still, rebuild owned resources on restoration and resume from the current state. Check another view before releasing any shared texture.

## Sources

- [MDN WebGL practices](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/WebGL_best_practices): per-pixel budgets, buffer size and resource costs.
- [Intersection Observer](https://developer.mozilla.org/en-US/docs/Web/API/Intersection_Observer_API): proximity and visibility relative to the observed root, including preparation margins.
- [Page Visibility API](https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API): document visibility changes, distinct from whether a region is in view.
- [`requestAnimationFrame`](https://developer.mozilla.org/en-US/docs/Web/API/Window/requestAnimationFrame): one-shot scheduling and timestamp-based progress across refresh rates.
- [WebGL 2 specification](https://registry.khronos.org/webgl/specs/latest/2.0/): context loss, restoration and implementation limits.
