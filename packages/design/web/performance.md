+++
description = "Use it to measure a web performance problem on the emitted page and the affected journey and to find the real bottleneck."
uses = [{ scope = "local", resource_id = "web" }]
see_also = [{ scope = "local", resource_id = "type" }, { scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "animate/profile-a-frame" }, { scope = "local", resource_id = "figures/debug-the-drawing-layer" }]

[guide]
exposure = "internal"
+++

# Measure a web performance problem where it occurs

## Measure the actual bottleneck

Use the emitted page and affected journey. Identify the actual largest paint and request chain rather than assuming the biggest source image is responsible. Avoid delaying that candidate through lazy or late discovery. Reserve media geometry where loading would otherwise move content. Font delivery can change layout and identity; use [Make typography carry the intended meaning](../type.md).

For an interaction, distinguish input delay, handler work and presentation delay. Apply immediate feedback, then yield or split nonessential work with the native scheduler when appropriate. A pressed style cannot repair a blocked main thread. Look for forced synchronous layout and broad per-frame style invalidation. Measure actual properties and area; transform or opacity alone does not guarantee cheap execution. Use [Design motion that helps people use the interface](../animate.md) for its timing and lifecycle.

Record device, viewport, relevant network/CPU conditions and the actual metric or trace. Lab diagnosis and field distributions establish different things. Use current [Core Web Vitals definitions](https://web.dev/articles/vitals) and applicable thresholds. Do not call every late animation frame an observed dropped frame or a lab proxy field INP. Missing field data does not imply no defect, and a score is not a creative verdict. Recheck the original symptom and affected consumers proportionately. Record the previous symptom, what the trace shows and whether the reader can act sooner. Operation, visual judgment and acceptance stay distinct.

## Make the moving image pay for itself in the actual page

Name the first paint, first usable action and sustained motion the task needs. Keep the resting still or poster available before enhancement. Prioritize it when it is the actual first-view largest-paint candidate; `fetchpriority="high"` is a hint whose effect needs the request trace. Below-fold media follows the page's loading budget. Measure transfer, mount work, first-start allocations and steady motion separately: a fast mount can defer an expensive shader compile or texture upload to first use.

Keep per-frame writes local to the moving element instead of changing an inherited property on the document root. A registered `@property` with `inherits: false` can limit inheritance where that fits the mechanism. Use `will-change` for a measured need and release it when the movement ends or is cancelled. Moving a GPU layer still costs memory and composition time. [Profile and repair motion in the browser](../animate/profile-a-frame.md) supplies the diagnostic method.

For long offscreen sections, test `content-visibility: auto` with an appropriate `contain-intrinsic-size` when the target supports it. Inspect resulting scroll stability, find, fragment navigation and focus rather than treating skipped rendering as free. Essential first-view content and controls still need to appear promptly.

For scrubbed video, compare short, regular keyframe intervals and encoded size on the target's actual decoder. Seeking generally decodes forward from a preceding random-access frame; a small download can still seek poorly. Independent frames are another option when frame-exact reverse scrubbing justifies their transfer and memory cost. Inspect the encoded frame listing and test forward and reverse seeks and the authored holds. Put the measured interval and its reason with the delivered asset rather than prescribing one interval for every film.

Keep a bounded working set: decode the current beat, prepare the next before it is reached, and release distant beats as the reader moves. Size that window for direction changes and measured latency instead of downloading every long sequence on entry. `preload` is a browser hint, not proof of readiness. Use `HTMLImageElement.decode()` for an image sequence where appropriate; readiness for drawing on a GPU can require upload as well as decode. Test the first frame of each beat for stalls.

Budget the decoded and drawn pixels. An RGBA8 frame's base storage is width × height × 4 bytes before additional buffers, formats or mipmaps; compressed bytes do not describe residency. Size the drawing buffer to the region times a measured, capped pixel ratio, not to the entire screen by default. [Spend rendering work where the reader can see it](../figures/debug-the-drawing-layer.md) covers the fuller resource accounting. Let a passive scroll listener record position and a frame loop draw the required frame; stop when it already matches, suspend offscreen or hidden work, and release resources that are no longer needed.

Record bytes per beat, peak resident frames and estimated memory, frame cost during rapid scrubbing and any boundary stall on the relevant phone and desktop. Keep untouched measurements marked not run. A corrective reduction in quality still has to preserve the approved artwork, transitions and reading holds.

## Worked example

A click visibly waits on a long page. Trace from input through the event handler, layout, paint and the next visible state. If code reads geometry and writes styles for every pointer event, compare a frame-coalesced update. Check the same interaction, with the same measurement conditions, after the change.

## Sources

- [Web Vitals](https://web.dev/articles/vitals): current metrics, thresholds, percentile assessment and the distinction between field and lab measurements.
- [Optimize LCP](https://web.dev/articles/optimize-lcp), [INP](https://web.dev/articles/optimize-inp) and [CLS](https://web.dev/articles/optimize-cls): diagnose resource discovery, input phases and layout shifts separately.
- [MDN `@property`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@property) and [`will-change`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/will-change): inheritance registration and the cost of retaining a rendering hint.
- [MDN image decode](https://developer.mozilla.org/en-US/docs/Web/API/HTMLImageElement/decode): decoding before insertion and handling rejection.
- [FFmpeg codec options](https://ffmpeg.org/ffmpeg-codecs.html): encoder-specific keyframe controls; inspect the actual encoded result.
- [MDN WebGL practices](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/WebGL_best_practices): drawing-buffer size, per-pixel budgets and resource lifetime.
