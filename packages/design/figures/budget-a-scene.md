+++
description = "Use it to choose whether a drawing layer is a live renderer or a still, pose set or film, and to budget it."
uses = [{ scope = "local", resource_id = "figures" }]
see_also = [{ scope = "local", resource_id = "figures/webgl" }]

[guide]
exposure = "internal"
+++

# Choose and budget a drawing layer

Choose the form the audience needs. The production source can be a rig, procedural field or 3D scene while the delivered result is a still, a pose set or a film. A live renderer earns its place when the interaction depends on geometry, responding light, continuous deformation or the relationship between passes.

Inspect a selected source before replacing its renderer. Trace the geometry, materials, occlusion, persistent objects and coordinated passes that carry its character. A field can depend on spatial structure even when its final image looks flat. Preserve authored shaders and postprocessing that serve the approved direction; distinguish them from accidental demo palettes, default materials or bloom. Keep the editable source and its provenance with any rendered derivative.

| Layer | Use when | Watch for |
| --- | --- | --- |
| CSS | Planes, masks, gradients, state images and a few transformable objects | Many elements or work that needs pixel sampling |
| SVG | Countable paths, marks and diagrams | Thousands of live nodes at frame rate |
| Canvas 2D | Pose sheets, per-object plotting or modest particle fields | Large textures or work that belongs per pixel |
| Small WebGL layer, such as OGL or the platform API | Textured planes, dissolves or fragment effects | Building a scene graph by hand to gain cameras and lights |
| 2D GPU compositor, such as PixiJS | Many sprites, masks and video textures share a drawing path | Repeated uploads or a second engine doing the same work |
| Scene graph, such as three.js | Geometry, camera, materials, shadows or coordinated passes determine the result | Unmeasured memory, default materials or excessive passes |
| Authored player | A vector figure has jointly authored states and transitions | Its own unreviewed colors, timing and fallback |
| WebGPU | A measured limit warrants its compute model | An unavailable device and an untested fallback |

These are capabilities, not a compulsory production order. CSS can turn a few flat objects in depth with perspective and 3D transforms; a pose sheet drawn through Canvas 2D's `drawImage()` can select a whole frame or a source rectangle. Compare those forms where they preserve the needed change. A live scene is justified by what the audience can perceive or control, not by the word “3D” in its source. Choose the drawing layer separately from [the sequence clock](../films/choose-the-clock.md). A film adaptation must verify readiness, randomness and accumulated simulation against independent frame seeking; sharing a browser renderer does not establish that contract.

Measure the busiest composition at target resolution on a mid-range phone and a desktop. Count draw calls, visible triangles, resident textures and frame cost. For a film also measure capture cost and whether independently seeking a frame is deterministic. Instance repeated geometry and material. Use a cheaper distant model and correct object bounds for culling.

Record built bytes after bundling, including helpers, decoders, fonts and media, rather than the library's advertised minimum. Inspect imports against the target's real module-loading rules before building the integration. Count GPU contexts and coordinated passes as one delivered system; several supporting packages may form one renderer, while two engines that duplicate the same output need justification. Compare first-frame compilation and uploads, sustained busy work and the return to an idle state. Report a device that was not measured as unmeasured.

Texture file size is not resident memory. Estimate from dimensions, format and mip levels, then inspect the actual allocation; compressed GPU textures and decoded image files have different costs. Include render targets and temporary passes. Keep required assets available through the destination's declared delivery route and verify supported formats and decoder loading there. A transport limitation belongs with that interface's owner, rather than silently weakening an approved composition.

Use unlit surfaces for flat color, a simple lit material for matte surfaces, and a physically based material where its response matters. An environment map may replace several lights. Each shadow-casting light renders another depth map. Paint or bake a shadow if the light, ground and object do not change in relation. For a changing scene, size the live map to visible detail and measure its pass. Ship any required decoder and environment with the page when it must work independently of a service.

Leave the editable master with its needed tracks, rigs and cameras. Strip unused parts from delivery copies after validating them in the renderer. Make every pointer-picked object reachable by keyboard and touch using the DOM. On teardown release geometry, materials, textures, render targets and renderer. Check resources created by a loader that no longer appear in the current scene. Use the [Run and recover a WebGL drawing region](webgl.md) for sizing, draw scheduling and recovery.

A library's orbit, fly or transform controls bring their own keys and response. Bind adopted controls to the interface's input and motion rules and inspect their camera framing; do not accept their defaults as a design decision. Check the drawn view, composed still, no-script document and failed-context state. Canvas pixels do not automatically adopt forced colors or other page presentation preferences, so redraw an appropriate form or expose the useful alternative.

Removing an object from the scene does not free its GPU resources. Track ownership and release a shared texture or geometry only after its last consumer finishes. Reopen and replace the scene several times and inspect whether owned resources accumulate. Keep teardown and context recovery in the renderer's lifecycle, with a usable still while recovery is unavailable.

Put the chosen layer and what a simpler delivery would lose in the existing design or production plan. Include the cost of replacing an already vetted renderer; a lower-level rewrite is not automatically smaller or more reliable. Use the actual dependency versions and their documentation. [OGL](https://github.com/oframe/ogl) supplies a small WebGL wrapper, [PixiJS](https://pixijs.com/) supplies a 2D renderer, and [three.js](https://threejs.org/) supplies a scene graph. These are examples of capabilities, not defaults for every scene.

## Worked example

A live mechanism has many identical bolts and one moving gear. Share the bolt geometry and material and draw instances. Paint the static support shadow. Keep a small live shadow for the gear only if that response shows its separation from the ground. Compare one still or a pose set with the interactive version. If continuous angle and occlusion matter, keep the scene graph. Record the draw calls and slowest busy frames. Remove and reopen the figure and check that textures and targets do not keep growing.

## Sources

- [MDN WebGL practices](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/WebGL_best_practices): context economy, buffer sizing, allocation and avoiding driver stalls when budgeting the GPU path.
- [Canvas `drawImage()`](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/drawImage) and [CSS transforms](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Transforms/Using): whole-frame or source-region delivery and flat objects placed in depth before requiring live scene geometry.
- [PixiJS textures](https://pixijs.com/8.x/guides/components/textures): texture sources, prepared uploads and shared views in the 2D compositor.
- [three.js textures](https://threejs.org/manual/pages/textures.html) and [cleanup](https://threejs.org/manual/pages/cleanup.html): resident texture cost and explicit disposal of scene resources.
- [WebGPU specification](https://www.w3.org/TR/webgpu/): adapter capabilities, limits and device loss when evaluating a compute or rendering path; verify the chosen implementation and fallback separately.
