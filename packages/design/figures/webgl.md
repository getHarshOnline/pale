+++
description = "Use it to start, size and recover a WebGL drawing region, from the drawing buffer and pixel ratio to losing and restoring the context."
uses = [{ scope = "local", resource_id = "figures" }]
official_sources = [{ label = "Run and recover a WebGL drawing region", url = "https://registry.khronos.org/webgl/specs/latest/2.0/" }]

[guide]
exposure = "internal"

[stack]
+++

# Run and recover a WebGL drawing region

Start when the region is near use, after its text and controls are usable. Choose alpha, antialiasing and color settings for the composition. Size the drawing buffer from the observed region and the supported pixel ratio, rounded to whole pixels. Observe the element’s resize. A window resize event alone misses layout changes within a viewport. Use the region size rather than the full window.

Cap the drawing buffer’s pixel ratio according to the project’s measured quality budget instead of multiplying blindly by a high device ratio. Compare the lowest supported buffer size with the intended composition. For distance-based shader edges, blend a small band with `smoothstep` instead of a hard `step` where the softer edge preserves the design. Inspect that edge at the lowest quality setting; more pixels cannot compensate for an incorrectly drawn threshold.

Use `ResizeObserver` for the region, with device-pixel content-box measurements where supported or a checked CSS-size conversion otherwise. Resize the buffer only when its dimensions change. Use a separate near-viewport observation to prepare expensive resources when the delivery budget allows; its callback should schedule initialization, not perform an unbounded compile on the main thread.

Compile the shaders and link the program before querying individual compile status. Eager `COMPILE_STATUS` reads can wait for the driver and serialize work. Where `KHR_parallel_shader_compile` is available, poll `COMPLETION_STATUS_KHR` on later frames until complete, then inspect `LINK_STATUS`. Without it, the link-status check can block, so measure initialization cost. On link failure, inspect program and shader logs for diagnosis and retain the composed still. Do not repeat status queries in the drawing loop. A deliberate broken-shader check should leave useful content visible rather than a blank region.

## Worked example: stop drawing while the context is lost

```js
canvas.addEventListener("webglcontextlost", (event) => {
  event.preventDefault();
  stopDrawing();
  showComposedStill();
});
canvas.addEventListener("webglcontextrestored", () => {
  if (rebuildResources()) {
    drawCurrentState();
    showCanvas();
  }
});
```

The renderer owns the helper calls, including a failed rebuild that keeps the still. After restoration recreate shaders, geometry, textures and framebuffers. Treat the old GL objects as invalid. Use `WEBGL_lose_context` for a controlled check where available. Allocate once where possible, dispose owned objects on teardown and keep shared assets alive while any consumer uses them.

Track passes, resolution, live textures and main-thread work. A smaller target can serve a blurred pass. Follow the real clock’s timestamp: fixed increments per frame differ at 60 and 120 Hz. Stop or coalesce drawing when nothing the reader can see changes. Distinguish texture color space and renderer output before regrading a washed-out image. Check a matched still and the first live frame. Keep the reading and input paths in the surrounding interface.

## Draw once when an idle figure becomes visible

```js
let visible = false;
let frame = null;
function requestDraw() {
  if (!visible || document.hidden || frame !== null) return;
  frame = requestAnimationFrame((time) => {
    frame = null;
    if (visible && !document.hidden) drawCurrentState(time);
  });
}
const observer = new IntersectionObserver(([entry]) => {
  visible = entry.isIntersecting;
  requestDraw();
});
observer.observe(mount);
document.addEventListener("visibilitychange", requestDraw);

function detach() {
  observer.disconnect();
  document.removeEventListener("visibilitychange", requestDraw);
  if (frame !== null) cancelAnimationFrame(frame);
}
```

Call `requestDraw()` after a data change, input or resize. An idle figure has no steady loop. Use the appropriate clock and pause behavior for a figure whose visible state evolves over time.

## Sources

- [MDN WebGL practices](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/WebGL_best_practices): shader query stalls, buffer sizing, allocation and disposal.
- [Parallel shader compilation](https://developer.mozilla.org/en-US/docs/Web/API/KHR_parallel_shader_compile): nonblocking completion polling and its distinction from link success.
- [MDN `ResizeObserver`](https://developer.mozilla.org/en-US/docs/Web/API/ResizeObserver): observing element size rather than only window changes.
- [Context restoration](https://developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/webglcontextrestored_event): rebuilding invalidated resources after restoration.
- [The Book of Shaders: Shapes](https://thebookofshaders.com/07/): distance-based shapes and smoothed threshold edges.
